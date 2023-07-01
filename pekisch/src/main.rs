#![doc = include_str!("../README.md")]

use std::{
    collections::HashSet,
    sync::{Arc, Condvar, Mutex},
};

use anyhow::Context;
use clap::{command, Parser};
use client_utils::{
    announce_as_publisher, announce_protocol_version, create_client, create_watchdog,
};
use futures_util::SinkExt;
use klib::core::{
    chord::Chord,
    note::{HasNoteId, Note},
};
use midly::MidiMessage;
use morivar::{PublisherToServer, ServerToPublisher, ToMessage};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
};
use tokio::{select, task::spawn_blocking};
use tokio_websockets::WebsocketStream;
use tracing::{info, warn};
use watchdog::{Expired, Signal};

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
    let secure = args.secure;

    let uri = client_utils::create_uri(args.url, args.secure)?;

    loop {
        let pair = Arc::new((Mutex::new(false), Condvar::new()));

        let (midi_tx, mut midi_rx) = mpsc::channel(midi_event_queue_length);

        let midi_events = spawn_blocking(move || {
            forward(midi_tx.clone(), device, Arc::clone(&pair)).context("Failed to forward MIDI")
        });

        let error = loop {
            info!("Attempting to connect to server");
            let mut stream = match create_client(&uri, secure).await {
                Ok(stream) => stream,
                Err(e) => {
                    warn!("{e:?}");
                    break e;
                }
            };
            if let Err(e) = handle_client(&mut stream, &mut midi_rx, &args.id, args.pingpong).await
            {
                break e;
            }
        };
        tokio::join!(midi_events).0??;
        warn!("Failed to handle connection: {error:?}");
        tokio::time::sleep(morivar::CLIENT_RECONNECT_DURATION).await;
    }
}

async fn handle_client<S>(
    stream: &mut WebsocketStream<S>,
    midi_rx: &mut mpsc::Receiver<MidiMessage>,
    id: &str,
    pingpong: bool,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    announce_protocol_version(stream).await?;

    announce_as_publisher(id, stream).await?;

    let mut notes = HashSet::new();

    let (mut interval, watchdog, mut expiration) = create_watchdog().await?;

    loop {
        select! {
            msg = stream.next() => {
                let Some(Ok(msg)) = msg else {
                    anyhow::bail!("Error receiving message: {msg:?}");
                };
                let Ok(msg) = msg.as_text() else {
                   anyhow::bail!("Expected text message, got: {msg:?}");
                };
                let Ok(ServerToPublisher::Pong) = serde_json::from_str(msg) else {
                    anyhow::bail!("Expected Pong, got: {msg:?}");
                };
                watchdog.send(Signal::Stop).await.context("Failed to stop watchdog")?;
            }
            event = midi_rx.recv() => {
                let Some(event) = event else {
                    break;
                };
                handle_midi_event(event, &mut notes);
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
            _i = interval.tick(), if pingpong => {
                info!("Sending Ping!");
                watchdog.send(Signal::Reset).await?;
                stream.send(PublisherToServer::Ping.to_message()).await?;
            }
            e = &mut expiration, if pingpong => {
                let Expired = e.context("Failed to monitor watchdog")?;
                anyhow::bail!("Server failed to pong");
            }
        }
    }

    info!("No more MIDI events, closing piano client");
    stream.close(None, None).await?;
    Ok(())
}

fn handle_midi_event(event: MidiMessage, notes: &mut HashSet<Note>) {
    match event {
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
}
