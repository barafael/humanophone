#![doc = include_str!("../README.md")]

use std::time::Duration;

use anyhow::Context;
use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
use klib::core::{
    base::{Playable, PlaybackHandle},
    named_pitch::NamedPitch,
    note::Note,
    octave::Octave,
};
use morivar::{ConsumerToServer, ServerToConsumer, ToMessage};
use once_cell::sync::Lazy;
use pitches::Pitches;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    select,
};
use tokio_websockets::{ClientBuilder, WebsocketStream};
use tracing::{info, warn};
use watchdog::{Expired, Signal, Watchdog};

mod pitches;

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[command(flatten)]
    args: morivar::cli::ClientArguments<{ env!("CARGO_BIN_NAME") }>,

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

    if play_jingle {
        jingle(&*ABEGG)?;
    }

    let uri = Uri::builder()
        .scheme(if args.secure { "wss" } else { "ws" })
        .authority(args.url)
        .path_and_query("/")
        .build()?;

    loop {
        info!("Attempting to connect to server");
        let stream = if args.secure {
            let connector = native_tls::TlsConnector::builder().build()?;
            let connector = tokio_websockets::Connector::NativeTls(connector.into());

            ClientBuilder::from_uri(uri.clone())
                .connector(&connector)
                .connect()
                .await?
        } else {
            ClientBuilder::from_uri(uri.clone()).connect().await?
        };

        if let Err(e) = handle_connection(stream, &args.id, args.pingpong).await {
            warn!("Failed to handle connection: {e:?}");
            tokio::time::sleep(morivar::CLIENT_RECONNECT_DURATION).await;
        }
    }
}

async fn handle_connection<S>(
    mut stream: WebsocketStream<S>,
    id: &str,
    pingpong: bool,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    info!("Announcing protocol version");
    let version = ConsumerToServer::ProtocolVersion(morivar::PROTOCOL_VERSION);
    stream.send(version.to_message()).await?;

    info!("Announcing as consumer");
    let announce = ConsumerToServer::IAmConsumer { id: id.to_string() };
    stream.send(announce.to_message()).await?;

    let mut interval = tokio::time::interval(morivar::PING_INTERVAL);

    let (watchdog, mut expiration) =
        Watchdog::with_timeout(morivar::PING_TO_PONG_ALLOWED_DELAY).run();
    watchdog
        .send(Signal::Stop)
        .await
        .expect("It's the first message");

    let mut handle = None;
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
                handle = new_handle;
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
    drop(handle);

    stream.close(None, None).await?;
    Ok(())
}

fn handle_message(text: &str) -> anyhow::Result<Option<PlaybackHandle>> {
    let Ok(msg) = serde_json::from_str::<ServerToConsumer>(text) else {
        anyhow::bail!("Protocol error, expected text message, got {text:?}")
    };
    match msg {
        ServerToConsumer::ChordEvent(chord) => {
            let ph = chord.play(
                Duration::ZERO,
                Duration::from_secs(5),
                Duration::from_millis(500),
            )?;
            Ok(Some(ph))
        }
        ServerToConsumer::PitchesEvent(pitches) => {
            let ph = Pitches::from(pitches).play(
                Duration::ZERO,
                Duration::from_secs(5),
                Duration::from_millis(500),
            )?;
            Ok(Some(ph))
        }
        ServerToConsumer::Silence | ServerToConsumer::Pong => Ok(None),
    }
}
