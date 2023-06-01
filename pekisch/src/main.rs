#![doc = include_str!("../README.md")]

use std::{
    collections::HashSet,
    io::{stdin, stdout, Write},
};

use anyhow::Context;
use clap::{command, Parser, ValueHint};
use futures_util::SinkExt;
use http::{uri::Authority, Uri};
use klib::core::{
    chord::Chord,
    note::{HasNoteId, Note},
};
use midi_control::MidiMessage;
use midir::{Ignore, MidiInput};
use morivar::PublisherMessage;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tokio_native_tls::native_tls;
use tokio_websockets::ClientBuilder;
use tracing::{info, warn};

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(short, long, value_hint = ValueHint::Url, default_value = "0.0.0.0:8000")]
    url: Authority,

    /// The id to report to Quinnipak
    #[arg(short, long, default_value = "Pekisch")]
    id: String,

    /// The index of the midi device to use
    #[arg(long)]
    device: Option<usize>,

    /// MIDI channel capacity
    #[arg(long, default_value_t = 256)]
    midi_event_queue_length: usize,

    #[arg(short, long, default_value_t = false)]
    secure: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();

    let (midi_tx, mut midi_rx) = mpsc::channel(args.midi_event_queue_length);

    let midi_events = spawn_blocking(move || {
        harvest_midi_events(midi_tx.clone(), args.device).context("Failed to harvest MIDI")
    });

    let uri = Uri::builder()
        .scheme(if args.secure { "wss" } else { "ws" })
        .authority(args.url)
        .path_and_query("/")
        .build()?;

    let mut client = if args.secure {
        let connector = native_tls::TlsConnector::builder().build()?;
        let connector = tokio_websockets::Connector::NativeTls(connector.into());

        ClientBuilder::from_uri(uri)
            .connector(&connector)
            .connect()
            .await?
    } else {
        ClientBuilder::from_uri(uri).connect().await?
    };

    let announce = PublisherMessage::IAmPublisher { id: args.id };
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
        let message = if let Some(chord) =
            Chord::try_from_notes(notes.iter().copied().collect::<Vec<_>>().as_slice())
                .ok()
                .and_then(|chords| chords.first().cloned())
        {
            PublisherMessage::PublishChord(chord).to_message()
        } else {
            PublisherMessage::PublishPitches(notes.clone()).to_message()
        };
        client.send(message).await?;
    }

    info!("No more MIDI events, closing piano client");
    client.close(None, None).await?;

    tokio::join!(midi_events).0?
}

fn harvest_midi_events(
    midi_tx: mpsc::Sender<(u64, MidiMessage)>,
    index: Option<usize>,
) -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    // Get an input port (read from console if multiple are available)
    let in_ports = midi_in.ports();
    let in_port = match (in_ports.len(), index) {
        (0, _) => anyhow::bail!("No input port found"),
        (1, _) => {
            println!(
                "Choosing the only available input port: {}",
                midi_in
                    .port_name(&in_ports[0])
                    .context("Device disconnected")?
            );
            &in_ports[0]
        }
        (_, None) => {
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
        (n, Some(index)) if index < n => &in_ports[index],
        (n, Some(index)) => {
            anyhow::bail!("Invalid index {index}, there are only {n} devices");
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
                    warn!("Failed to forward midi message: {e}");
                }
            },
            (),
        )
        // Somehow this doesn't want to be a proper StdError:
        .map_err(|e| anyhow::format_err!("{e}"))
        .context("Failed to connect to midi input")?;

    println!("Connection open, reading input from '{in_port_name}'");
    println!("(press enter to exit) ...");

    stdin().read_line(&mut String::new())?; // wait for next enter key press

    println!("Closing connection");
    Ok(())
}
