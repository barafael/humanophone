use anyhow::Context;
use futures_util::SinkExt;
use morivar::{ConsumerToServer, ServerToConsumer, ToMessage};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::broadcast,
};
use tokio_websockets::{Message, WebsocketStream};
use tracing::info;
use watchdog::{Expired, Signal, Watchdog};

pub async fn run<S>(
    mut chords_receiver: broadcast::Receiver<ServerToConsumer>,
    mut stream: WebsocketStream<S>,
    pingpong: bool,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (watchdog, mut expired) = Watchdog::with_timeout(morivar::PING_AWAIT_INTERVAL).run();
    loop {
        tokio::select! {
            event = chords_receiver.recv() => {
                let m = event.context("Failed to receive message on internal chord broadcast")?;
                stream.send(m.to_message()).await?;
            }
            item = stream.next() => {
                match item {
                    Some(Ok(ref msg)) => {
                        let response = handle_consumer_message(msg)?;
                        watchdog.send(Signal::Reset).await?;
                        stream.send(response.to_message()).await?;
                    }
                    Some(e) => {
                        e.context("Error on websocket client connection")?;
                    }
                    None => {
                        info!("Stream ended!");
                        break;
                    }
                }
            }
            e = &mut expired, if pingpong => {
                let Expired = e.context("Failed to monitor watchdog")?;
                anyhow::bail!("Consumer failed to ping");
            }
        }
    }
    Ok(())
}

fn handle_consumer_message(msg: &Message) -> anyhow::Result<ServerToConsumer> {
    if matches!(
        msg.as_text().map(serde_json::from_str),
        Ok(Ok(ConsumerToServer::Ping))
    ) {
        info!("Sending Pong");
        Ok(ServerToConsumer::Pong)
    } else {
        // TODO limit message length perhaps.
        anyhow::bail!("Expected ConsumerToServer::Ping, got: {msg:?}");
    }
}
