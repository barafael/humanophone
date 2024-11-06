use anyhow::Context;
use morivar::{PublisherToServer, ServerToConsumer, ServerToPublisher, ToMessage};

use either::{Either as Response, Left as Forward, Right as ReturnToSender};

use futures_util::SinkExt;
use simple_tokio_watchdog::{Expired, Signal, Watchdog};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    select,
    sync::broadcast,
};
use tokio_websockets::{Message, WebsocketStream};
use tracing::{info, warn};

pub async fn run<S>(
    chords_sender: broadcast::Sender<ServerToConsumer>,
    mut stream: WebsocketStream<S>,
    pingpong: bool,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (watchdog, mut expired) = Watchdog::with_timeout(morivar::PING_AWAIT_INTERVAL).run();
    loop {
        select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(ref msg)) => {
                        watchdog.send(Signal::Reset).await?;
                        match handle_message(msg) {
                            Forward(consumer_message) => {
                                if let Err(c) = chords_sender.send(consumer_message) {
                                    warn!("Currently no subscribed consumers, dropping {:?}", c.0);
                                }
                            }
                            ReturnToSender(publisher_message) => {
                                stream.send(publisher_message.to_message()).await?;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        warn!("Reading from stream produced error: {e:?}");
                    }
                    None => {
                        info!("Publisher stream closed");
                        return Ok(())
                    }
                }
            },
            e = &mut expired, if pingpong => {
                let Expired = e.context("Failed to monitor watchdog")?;
                anyhow::bail!("Publisher failed to ping");
            }
        }
    }
}

fn handle_message(msg: &Message) -> Response<ServerToConsumer, ServerToPublisher> {
    let Ok(text) = msg.as_text() else {
        return ReturnToSender(ServerToPublisher::Error(
            "Only text messages allowed".into(),
        ));
    };
    match serde_json::from_str(text) {
        Ok(PublisherToServer::PublishChord(chord)) => {
            info!("{chord:?}");
            Forward(ServerToConsumer::ChordEvent(chord))
        }
        Ok(PublisherToServer::PublishPitches(pitches)) => {
            info!("Pitches: {pitches:?}");
            Forward(ServerToConsumer::PitchesEvent(pitches))
        }
        Ok(PublisherToServer::PublishSilence) => Forward(ServerToConsumer::Silence),
        Ok(PublisherToServer::Ping) => ReturnToSender(ServerToPublisher::Pong),
        e => ReturnToSender(ServerToPublisher::Error(format!(
            "Deserialization failed: {e:?}"
        ))),
    }
}
