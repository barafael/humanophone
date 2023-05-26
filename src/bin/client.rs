use futures_util::SinkExt;
use http::Uri;
use humanophone::ConsumerMessage;
use tokio_websockets::ClientBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let uri = Uri::from_static("ws://127.0.0.1:3000");
    let mut client = ClientBuilder::from_uri(uri).connect().await?;

    let announce = ConsumerMessage::IAmConsumer {
        id: "I want some chords".to_string(),
    };
    client.send(announce.to_message()).await?;

    while let Some(Ok(msg)) = client.next().await {
        println!("chord: {msg:?}");
    }

    client.close(None, None).await?;
    Ok(())
}
