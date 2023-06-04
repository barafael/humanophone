use anyhow::Context;
use futures_util::SinkExt;
use morivar::ConsumerMessage;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::broadcast,
};
use tokio_websockets::{Message, WebsocketStream};
use tracing::info;
use watchdog::{Reset, Watchdog};

pub async fn handle_consumer<S>(
    mut chords_receiver: broadcast::Receiver<ConsumerMessage>,
    mut stream: WebsocketStream<S>,
    pingpong: bool,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (resetter, mut expired) = Watchdog::with_timeout(morivar::PING_AWAIT_INTERVAL).spawn();
    loop {
        tokio::select! {
            event = chords_receiver.recv() => {
                let m = event.context("Failed to receive message on internal chord broadcast")?;
                stream.send(m.to_message()).await?;
            }
            item = stream.next() => {
                match item {
                    Some(Ok(msg)) => {
                        let response = handle_message(msg)?;
                        resetter.send(Reset::Signal).await?;
                        stream.send(response.to_message()).await?;
                    }
                    Some(Err(e)) => {
                        anyhow::bail!("Error on websocket client connection: {e:?}")
                    }
                    None => {
                        info!("Stream ended!");
                        break;
                    }
                }
            }
            e = &mut expired, if pingpong => {
                match e {
                    Ok(_expired) => {
                        anyhow::bail!("Consumer failed to ping")
                    }
                    Err(e) => anyhow::bail!("Failed to monitor watchdog: {e:?}")
                }
            }
        }
    }
    Ok(())
}

fn handle_message(msg: Message) -> anyhow::Result<ConsumerMessage> {
    if let Ok(Ok(ConsumerMessage::Ping)) = msg.as_text().map(serde_json::from_str) {
        info!("Sending Pong");
        Ok(ConsumerMessage::Pong)
    } else {
        // TODO limit message length perhaps.
        anyhow::bail!("Invalid consumer message: {msg:?}");
    }
}
