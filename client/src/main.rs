use futures_util::SinkExt;
use http::Uri;
use humanophone_server::ConsumerMessage;
use tokio_websockets::ClientBuilder;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let uri = Uri::from_static("wss://0.0.0.0:8000");
    let mut client = ClientBuilder::from_uri(uri).connect().await?;

    let announce = ConsumerMessage::IAmConsumer {
        id: "I want some chords".to_string(),
    };
    client.send(announce.to_message()).await?;

    loop {
        let next = client.next().await;
        if let Some(Ok(msg)) = next {
            if let Ok(text) = msg.as_text() {
                if let Ok(ConsumerMessage::ChordEvent(notes, chord)) = serde_json::from_str(text) {
                    info!("{notes:?}");
                    if let Some(chord) = chord {
                        info!("{chord}");
                    }
                }
            } else {
                warn!("Stopping receive");
                break;
            }
        } else {
            warn!("Breaking on client message: {next:?}");
            break;
        }
    }

    client.close(None, None).await?;
    Ok(())
}
