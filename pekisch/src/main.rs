#![doc = include_str!("../README.md")]

use std::{
    collections::HashSet,
    sync::{Arc, Condvar, Mutex},
};

use anyhow::Context;
use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
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
use tokio_native_tls::native_tls;
use tokio_websockets::{ClientBuilder, WebsocketStream};
use tracing::{info, warn};
use watchdog::{Expired, Signal, Watchdog};

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

    let uri = Uri::builder()
        .scheme(if args.secure { "wss" } else { "ws" })
        .authority(args.url)
        .path_and_query("/")
        .build()?;

    loop {
        let pair = Arc::new((Mutex::new(false), Condvar::new()));

        let (midi_tx, midi_rx) = mpsc::channel(midi_event_queue_length);

        let midi_events = spawn_blocking(move || {
            forward(midi_tx.clone(), device, Arc::clone(&pair)).context("Failed to forward MIDI")
        });

        let stream = if args.secure {
            let connector = native_tls::TlsConnector::builder().build()?;
            let connector = tokio_websockets::Connector::NativeTls(connector.into());

            ClientBuilder::from_uri(uri.clone())
                .connector(&connector)
                .connect()
                .await
        } else {
            ClientBuilder::from_uri(uri.clone()).connect().await
        }
        .context("Failed to connect to server")?;

        if let Err(e) = handle_connection(stream, midi_rx, &args.id, args.pingpong).await {
            warn!("Failed to handle connection: {e:?}");
            tokio::time::sleep(morivar::CLIENT_RECONNECT_DURATION).await;
        }
        tokio::join!(midi_events).0??;
    }
}

async fn handle_connection<S>(
    mut stream: WebsocketStream<S>,
    mut midi_rx: mpsc::Receiver<MidiMessage>,
    id: &str,
    pingpong: bool,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let version = PublisherToServer::ProtocolVersion(morivar::PROTOCOL_VERSION);
    stream.send(version.to_message()).await?;

    let announce = PublisherToServer::IAmPublisher { id: id.to_string() };
    stream.send(announce.to_message()).await?;

    let mut notes = HashSet::new();

    let mut interval = tokio::time::interval(morivar::PING_INTERVAL);

    let (watchdog, mut expiration) =
        Watchdog::with_timeout(morivar::PING_TO_PONG_ALLOWED_DELAY).run();
    watchdog
        .send(Signal::Stop)
        .await
        .expect("It's the first message");

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
                handle_midi_event(&event, &mut notes);
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

fn handle_midi_event(event: &MidiMessage, notes: &mut HashSet<Note>) {
    match event {
        MidiMessage::NoteOn { key, vel } => {
            if let Ok(note) = Note::from_id(1u128 << key.as_int()) {
                if vel == &0 {
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
