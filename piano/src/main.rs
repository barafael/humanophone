use std::{
    collections::HashSet,
    io::{stdin, stdout, Write},
};

use anyhow::Context;
use futures_util::SinkExt;
use http::Uri;
use humanophone_server::PublisherMessage;
use klib::core::{
    chord::Chord,
    note::{HasNoteId, Note},
};
use midi_control::MidiMessage;
use midir::{Ignore, MidiInput};
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tokio_websockets::ClientBuilder;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let (midi_tx, mut midi_rx) = mpsc::channel(256);

    let t1 = spawn_blocking(|| harvest_midi_events(midi_tx));

    let uri = Uri::from_static("wss://0.0.0.0:8000");
    let mut client = ClientBuilder::from_uri(uri).connect().await?;

    let announce = PublisherMessage::IAmPublisher {
        id: "I am a piano".to_string(),
    };
    client.send(announce.to_message()).await?;

    let mut notes = HashSet::new();
    while let Some((_stamp, message)) = midi_rx.recv().await {
        match message {
            MidiMessage::NoteOn(_channel, event) => {
                if let Ok(note) = Note::from_id(1u128 << event.key) {
                    notes.insert(note);
                }
            }
            MidiMessage::NoteOff(_channel, event) => {
                if let Ok(note) = Note::from_id(1u128 << event.key) {
                    notes.remove(&note);
                }
            }
            MidiMessage::Invalid => {
                notes.clear();
            }
            e => {
                warn!("Unhandled MIDI message: {e:?}");
            }
        }
        let chord = Chord::try_from_notes(notes.iter().copied().collect::<Vec<_>>().as_slice())
            .ok()
            .and_then(|chords| chords.first().cloned());
        let message = PublisherMessage::PublishChord(notes.clone(), chord).to_message();
        client.send(message).await?;
    }

    info!("No more MIDI events, closing piano client");
    client.close(None, None).await?;

    tokio::try_join!(t1)?.0?;
    Ok(())
}

fn harvest_midi_events(midi_tx: mpsc::Sender<(u64, MidiMessage)>) -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    // Get an input port (read from console if multiple are available)
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => anyhow::bail!("No input port found"),
        1 => {
            println!(
                "Choosing the only available input port: {}",
                midi_in
                    .port_name(&in_ports[0])
                    .context("Device disconnected")?
            );
            &in_ports[0]
        }
        _ => {
            println!("\nAvailable input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!(
                    "{i}: {}",
                    midi_in.port_name(p).context("Device disconnected")?
                );
            }
            print!("Please select input port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin()
                .read_line(&mut input)
                .context("Failed to read line")?;
            in_ports
                .get(input.trim().parse::<usize>()?)
                .context("Invalid input port selected")?
        }
    };

    println!("\nOpening connection");
    let in_port_name = midi_in.port_name(in_port)?;

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let _conn_in = midi_in
        .connect(
            in_port,
            "humanophone-midi-in",
            move |stamp, message, _| {
                let message = MidiMessage::from(message);
                if let Err(e) = midi_tx.blocking_send((stamp, message)) {
                    warn!("Failed to forward midi message {e}");
                }
            },
            (),
        )
        // Somehow this doesn't want to be a proper StdError:
        .map_err(|e| anyhow::format_err!("{e}"))
        .context("Failed to connect to midi input")?;

    println!("Connection open, reading input from '{in_port_name}' (press enter to exit) ...");

    stdin().read_line(&mut String::new())?; // wait for next enter key press

    println!("Closing connection");
    Ok(())
}
