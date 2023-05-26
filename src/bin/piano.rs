use std::{
    collections::HashSet,
    io::{stdin, stdout, Write},
};

use futures_util::SinkExt;
use http::Uri;
use humanophone::PublisherMessage;
use klib::core::{
    chord::Chord,
    note::{HasNoteId, Note},
};
use midi_control::MidiMessage;
use midir::{Ignore, MidiInput};
use tokio::sync::mpsc;
use tokio_websockets::ClientBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (midi_tx, mut midi_rx) = mpsc::channel(256);

    let t1 = tokio::spawn(async move {
        run(midi_tx)?;
        Ok::<_, anyhow::Error>(())
    });

    let uri = Uri::from_static("ws://127.0.0.1:3000");
    let mut client = ClientBuilder::from_uri(uri).connect().await?;

    let announce = PublisherMessage::IAmPublisher {
        id: "I am a piano".to_string(),
    };
    client.send(announce.to_message()).await?;

    let mut notes = HashSet::new();
    while let Some((_stamp, message)) = midi_rx.recv().await {
        if let MidiMessage::NoteOn(_channel, event) = message {
            let note = Note::from_id(1u128 << event.key).unwrap();
            notes.insert(note);
        } else if let MidiMessage::NoteOff(_channel, event) = message {
            let note = Note::from_id(1u128 << event.key).unwrap();
            notes.remove(&note);
        }
        let chord = Chord::try_from_notes(notes.iter().cloned().collect::<Vec<_>>().as_slice())
            .ok()
            .and_then(|chords| chords.first().cloned());
        let message = PublisherMessage::PublishChord(notes.clone(), chord).to_message();
        client.send(message).await?;
    }

    client.close(None, None).await?;

    tokio::try_join!(t1)?.0?;
    Ok(())
}

fn run(midi_tx: mpsc::Sender<(u64, MidiMessage)>) -> anyhow::Result<()> {
    let mut input = String::new();

    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    // Get an input port (read from console if multiple are available)
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => anyhow::bail!("no input port found"),
        1 => {
            println!(
                "Choosing the only available input port: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
        _ => {
            println!("\nAvailable input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!("{}: {}", i, midi_in.port_name(p).unwrap());
            }
            print!("Please select input port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            in_ports
                .get(input.trim().parse::<usize>()?)
                .ok_or("invalid input port selected")
                .unwrap()
        }
    };

    println!("\nOpening connection");
    let in_port_name = midi_in.port_name(in_port)?;

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let _conn_in = midi_in
        .connect(
            in_port,
            "midir-read-input",
            move |stamp, message, _| {
                let message = MidiMessage::from(message);
                midi_tx.blocking_send((stamp, message)).unwrap();
            },
            (),
        )
        .unwrap();

    println!(
        "Connection open, reading input from '{}' (press enter to exit) ...",
        in_port_name
    );

    input.clear();
    stdin().read_line(&mut input)?; // wait for next enter key press

    println!("Closing connection");
    Ok(())
}
