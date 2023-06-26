#![doc = include_str!("../README.md")]

use std::{
    collections::HashSet,
    io::{stdin, stdout, Write},
};

use anyhow::Context;
use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
use klib::core::{
    chord::Chord,
    note::{HasNoteId, Note},
};
use midir::{Ignore, MidiInput};
use midly::{live::LiveEvent, MidiMessage};
use morivar::PublisherMessage;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tokio_native_tls::native_tls;
use tokio_websockets::ClientBuilder;
use tracing::{info, warn};

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[command(flatten)]
    args: morivar::cli::ClientArguments,

    /// The index of the midi device to use
    #[arg(long)]
    device: Option<usize>,

    /// MIDI channel capacity
    #[arg(long, default_value_t = 256)]
    midi_event_queue_length: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();
    let device = args.device;
    let midi_event_queue_length = args.midi_event_queue_length;
    let args = args.args;

    let (midi_tx, mut midi_rx) = mpsc::channel(midi_event_queue_length);

    let midi_events = spawn_blocking(move || {
        harvest_midi_events(midi_tx.clone(), device).context("Failed to harvest MIDI")
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

    let announce = PublisherMessage::IAmPublisher {
        id: args.id.unwrap_or_else(|| "Pekisch".to_string()),
    };
    client.send(announce.to_message()).await?;

    let mut notes = HashSet::new();
    while let Some(message) = midi_rx.recv().await {
        match message {
            MidiMessage::NoteOn { key, vel } => {
                if let Ok(note) = Note::from_id(1u128 << key.as_int()) {
                    if vel == 0 {
                        // It's a note-off, just hiding
                        notes.remove(&note);
                    } else {
                        notes.insert(note);
                    }
                }
            }
            MidiMessage::NoteOff { key, .. } => {
                if let Ok(note) = Note::from_id(1u128 << key.as_int()) {
                    notes.remove(&note);
                }
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
    midi_tx: mpsc::Sender<MidiMessage>,
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
            move |_stamp, message, _| {
                let Some(msg) = on_midi(message) else { return };
                if let Err(e) = midi_tx.blocking_send(msg) {
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

fn on_midi(event: &[u8]) -> Option<MidiMessage> {
    let event = LiveEvent::parse(event).unwrap();
    match event {
        LiveEvent::Midi { message, .. } => match message {
            msg @ (MidiMessage::NoteOn { .. } | MidiMessage::NoteOff { .. }) => Some(msg),
            _ => None,
        },
        _ => None,
    }
}
