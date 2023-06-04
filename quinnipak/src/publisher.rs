use morivar::{ConsumerMessage, PublisherMessage};

use either::{Either as Response, Left as Forward, Right as ReturnToSender};

use futures_util::SinkExt;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    select,
    sync::broadcast,
};
use tokio_websockets::{Message, WebsocketStream};
use tracing::{info, warn};
use watchdog::{Reset, Watchdog};

pub async fn handle_publisher<S>(
    chords_sender: broadcast::Sender<ConsumerMessage>,
    mut stream: WebsocketStream<S>,
    pingpong: bool,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (resetter, mut expired) = Watchdog::with_timeout(morivar::PING_AWAIT_INTERVAL).spawn();
    loop {
        select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        resetter.send(Reset::Signal).await?;
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
                        info!("Stream closed");
                        return Ok(())
                    }
                }
            },
            e = &mut expired, if pingpong => {
                match e {
                    Ok(_expired) => {
                        anyhow::bail!("Publisher failed to ping")
                    }
                    Err(e) => anyhow::bail!("Failed to monitor watchdog: {e:?}")
                }
            }
        }
    }
}

fn handle_message(msg: Message) -> Response<ConsumerMessage, PublisherMessage> {
    if let Ok(text) = msg.as_text() {
        match serde_json::from_str(text) {
            Ok(PublisherMessage::PublishChord(chord)) => {
                info!("{:?}", chord);
                Forward(ConsumerMessage::ChordEvent(chord))
            }
            Ok(PublisherMessage::PublishPitches(pitches)) => {
                info!("Pitches: {:?}", pitches);
                Forward(ConsumerMessage::PitchesEvent(pitches))
            }
            Ok(PublisherMessage::Silence) => Forward(ConsumerMessage::Silence),
            Ok(PublisherMessage::Ping) => ReturnToSender(PublisherMessage::Pong),
            Ok(PublisherMessage::IAmPublisher { id }) => {
                warn!("Publisher identified repeatedly, this time with {id}");
                ReturnToSender(PublisherMessage::NowAreYou)
            }
            Ok(m) => {
                let msg = format!("Invalid publisher message {m:?}");
                warn!(msg);
                ReturnToSender(PublisherMessage::InvalidMessage(msg))
            }
            Err(e) => ReturnToSender(PublisherMessage::InvalidMessage(format!(
                "Deserialization failed: {e:?}"
            ))),
        }
    } else {
        ReturnToSender(PublisherMessage::InvalidMessage(
            "Only text messages allowed".into(),
        ))
    }
}
