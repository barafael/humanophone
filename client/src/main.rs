use futures_util::SinkExt;
use http::Uri;
use humanophone_server::ConsumerMessage;
use tokio_websockets::ClientBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let uri = Uri::from_static("ws://127.0.0.1:3000");
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
                    dbg!(notes);
                    if let Some(chord) = chord {
                        dbg!(chord);
                    }
                }
            } else {
                dbg!("Stopping receive");
                break;
            }
        } else {
            dbg!(next);
            break;
        }
    }

    client.close(None, None).await?;
    Ok(())
}
