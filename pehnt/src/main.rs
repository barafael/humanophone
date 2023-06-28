#![doc = include_str!("../README.md")]

use std::time::Duration;

use anyhow::Context;
use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
use morivar::ConsumerMessage;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    select,
};
use tokio_websockets::{ClientBuilder, WebsocketStream};
use tracing::{info, warn};
use watchdog::{Expired, Signal, Watchdog};

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[command(flatten)]
    args: morivar::cli::ClientArguments<{ env!("CARGO_BIN_NAME") }>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse().args;

    let uri = Uri::builder()
        .scheme(if args.secure { "wss" } else { "ws" })
        .authority(args.url)
        .path_and_query("")
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
            tokio::time::sleep(Duration::from_millis(500)).await;
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
    let version = ConsumerMessage::ProtocolVersion(morivar::PROTOCOL_VERSION);
    stream.send(version.to_message()).await?;

    info!("Announcing as consumer");
    let announce = ConsumerMessage::IAmConsumer { id: id.to_string() };
    stream.send(announce.to_message()).await?;

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
                handle_message(text);
            }
            _i = interval.tick(), if pingpong => {
                info!("Sending Ping!");
                watchdog.send(Signal::Reset).await?;
                stream.send(ConsumerMessage::Ping.to_message()).await?;
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

fn handle_message(text: &str) {
    match serde_json::from_str(text) {
        Ok(ConsumerMessage::ChordEvent(chord)) => {
            info!("Chord: {chord}");
        }
        Ok(ConsumerMessage::PitchesEvent(pitches)) => {
            info!("Pitches: {pitches:?}");
        }
        Ok(ConsumerMessage::Silence) => {
            info!("SILENCE!!!");
        }
        Ok(ConsumerMessage::Pong) => {
            info!("Received Pong!");
        }
        Ok(m) => {
            warn!("Unhandled consumer message: {m:?}");
        }
        Err(e) => {
            warn!("Protocol error, expected ConsumerMessage: {e:?}");
        }
    }
}
