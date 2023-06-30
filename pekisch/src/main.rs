#![doc = include_str!("../README.md")]

use std::collections::HashSet;

use anyhow::Context;
use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
use klib::core::{
    chord::Chord,
    note::{HasNoteId, Note},
};
use midly::MidiMessage;
use morivar::{PublisherToServer, ToMessage};
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tokio_native_tls::native_tls;
use tokio_websockets::ClientBuilder;
use tracing::{info, warn};

use crate::midi::forward;

mod midi;

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[command(flatten)]
    args: morivar::cli::ClientArguments<{ env!("CARGO_BIN_NAME") }>,

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

    let midi_events =
        spawn_blocking(move || forward(midi_tx.clone(), device).context("Failed to harvest MIDI"));

    let uri = Uri::builder()
        .scheme(if args.secure { "wss" } else { "ws" })
        .authority(args.url)
        .path_and_query("/")
        .build()?;

    let mut stream = if args.secure {
        let connector = native_tls::TlsConnector::builder().build()?;
        let connector = tokio_websockets::Connector::NativeTls(connector.into());

        ClientBuilder::from_uri(uri)
            .connector(&connector)
            .connect()
            .await?
    } else {
        ClientBuilder::from_uri(uri).connect().await?
    };

    let version = PublisherToServer::ProtocolVersion(morivar::PROTOCOL_VERSION);
    stream.send(version.to_message()).await?;

    let announce = PublisherToServer::IAmPublisher { id: args.id };
    stream.send(announce.to_message()).await?;

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
            PublisherToServer::PublishChord(chord).to_message()
        } else {
            PublisherToServer::PublishPitches(notes.clone()).to_message()
        };
        stream.send(message).await?;
    }

    info!("No more MIDI events, closing piano client");
    stream.close(None, None).await?;

    tokio::join!(midi_events).0?
}
