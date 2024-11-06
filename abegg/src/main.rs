#![doc = include_str!("../README.md")]

use std::time::Duration;

use anyhow::Context;
use clap::{command, Parser};
use client_utils::{
    announce_as_consumer, announce_protocol_version, create_client, create_uri, create_watchdog,
};
use either::Either;
use futures_util::SinkExt;
use klib::core::{
    base::Playable, chord::Chord, named_pitch::NamedPitch, note::Note, octave::Octave,
};
use morivar::{ConsumerToServer, ServerToConsumer, ToMessage};
use once_cell::sync::Lazy;
use pitches::Pitches;
use simple_tokio_watchdog::{Expired, Signal};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    join, select,
    sync::mpsc,
    task::spawn_blocking,
};
use tokio_websockets::WebsocketStream;
use tracing::{info, warn};

mod pitches;
mod playback;

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[command(flatten)]
    args: morivar::cli::ClientArguments,

    /// Whether to play the ABEGG jingle
    #[arg(long, default_value_t = false)]
    jingle: bool,
}

static ABEGG: Lazy<[(Note, f32, f32); 5]> = Lazy::new(|| {
    [
        (Note::new(NamedPitch::A, Octave::Four), 0.1, 0.05),
        (Note::new(NamedPitch::B, Octave::Four), 0.07, 0.2),
        (Note::new(NamedPitch::E, Octave::Five), 0.1, 0.05),
        (Note::new(NamedPitch::G, Octave::Five), 0.1, 0.05),
        (Note::new(NamedPitch::G, Octave::Five), 0.1, 0.05),
    ]
});

fn jingle(events: &[(Note, f32, f32)]) -> anyhow::Result<()> {
    for (note, length, pause) in events {
        let _handle = note.play(
            Duration::ZERO,
            Duration::from_secs_f32(*length),
            Duration::from_millis(10),
        )?;
        std::thread::sleep(Duration::from_secs_f32(*length));
        std::thread::sleep(Duration::from_secs_f32(*pause));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();
    let play_jingle = args.jingle;
    let args = args.args;
    let secure = args.secure;
    let id = args.id;

    if play_jingle {
        jingle(&*ABEGG)?;
    }

    let uri = create_uri(args.url, args.secure)?;

    loop {
        let uri = uri.clone();
        let id = id.clone();

        tokio::spawn(async move {
            let (chord_tx, chord_rx) = mpsc::channel(32);

            let handle = spawn_blocking(move || playback::run(chord_rx));

            info!("Attempting to connect to server");
            let stream = create_client(&uri, secure).await?;

            abegg(stream, &id, args.pingpong, chord_tx).await?;
            join!(handle).0?;
            anyhow::Ok(())
        });
        tokio::time::sleep(client_utils::jittering_retry_duration()).await;
    }
}

/// Handle the client connection
async fn abegg<S>(
    mut stream: WebsocketStream<S>,
    id: &str,
    pingpong: bool,
    chords: mpsc::Sender<Either<Chord, Pitches>>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    announce_protocol_version(&mut stream).await?;

    announce_as_consumer(id, &mut stream).await?;

    let (mut interval, watchdog, mut expiration) = create_watchdog().await?;

    loop {
        select! {
            msg = stream.next() => {
                let Some(Ok(msg)) = msg else {
                    warn!("Breaking on client message: {msg:?}");
                    break;
                };
                let Ok(text) = msg.as_text() else {
                    warn!("Received non-text message, stopping receive");
                    break;
                };
                if pingpong {
                    // on any message, even non-pong, stop the watchdog - the server is alive at least.
                    watchdog.send(Signal::Stop).await.context("Failed to reset the watchdog")?;
                }
                let Ok(new_handle) = handle_message(text) else {
                    break
                };
                if let Some(either) = new_handle {
                    chords.send(either).await?;
                }
            }
            _i = interval.tick(), if pingpong => {
                info!("Sending Ping!");
                watchdog.send(Signal::Reset).await?;
                stream.send(ConsumerToServer::Ping.to_message()).await?;
            }
            e = &mut expiration, if pingpong => {
                let Expired = e.context("Failed to monitor watchdog")?;
                anyhow::bail!("Server failed to pong");
            }
        }
    }

    stream.close(None, None).await?;
    Ok(())
}

fn handle_message(text: &str) -> anyhow::Result<Option<Either<Chord, Pitches>>> {
    let Ok(msg) = serde_json::from_str::<ServerToConsumer>(text) else {
        anyhow::bail!("Protocol error, expected text message, got {text:?}")
    };
    match msg {
        ServerToConsumer::ChordEvent(chord) => Ok(Some(Either::Left(chord))),
        ServerToConsumer::PitchesEvent(pitches) => {
            let pitches = Pitches::from(pitches);
            Ok(Some(Either::Right(pitches)))
        }
        ServerToConsumer::Silence | ServerToConsumer::Pong => Ok(None),
    }
}
