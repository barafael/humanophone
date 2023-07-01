#![doc = include_str!("../README.md")]

use anyhow::Context;
use clap::{command, Parser};
use client_utils::{
    announce_as_consumer, announce_protocol_version, create_client, create_uri, create_watchdog,
};
use futures_util::SinkExt;
use morivar::{ConsumerToServer, ServerToConsumer, ToMessage};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    select,
};
use tokio_websockets::WebsocketStream;
use tracing::{info, warn};
use watchdog::{Expired, Signal};

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

    let uri = create_uri(args.url, args.secure)?;

    loop {
        info!("Attempting to connect to server");
        let mut stream = create_client(&uri, args.secure).await?;

        if let Err(e) = handle_connection(&mut stream, &args.id, args.pingpong).await {
            warn!("Failed to handle connection: {e:?}");
            tokio::time::sleep(morivar::CLIENT_RECONNECT_DURATION).await;
        }
    }
}

async fn handle_connection<S>(
    stream: &mut WebsocketStream<S>,
    id: &str,
    pingpong: bool,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    announce_protocol_version(stream).await?;

    announce_as_consumer(id, stream).await?;

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
                handle_message(text);
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

fn handle_message(text: &str) {
    match serde_json::from_str(text) {
        Ok(ServerToConsumer::ChordEvent(chord)) => {
            info!("Chord: {chord}");
        }
        Ok(ServerToConsumer::PitchesEvent(pitches)) => {
            info!("Pitches: {pitches:?}");
        }
        Ok(ServerToConsumer::Silence) => {
            info!("SILENCE!!!");
        }
        Ok(ServerToConsumer::Pong) => {
            info!("Received Pong!");
        }
        Err(e) => {
            warn!("Protocol error, expected ServerToConsumer: {e:?}");
        }
    }
}
