use std::collections::HashSet;

use futures_util::SinkExt;
use humanophone::{ConsumerMessage, PublisherMessage};
use klib::core::{chord::Chord, note::Note};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast,
};
use tokio_websockets::{ServerBuilder, WebsocketStream};
use tracing::warn;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:3000").await?;

    let (chords_tx, _) = broadcast::channel(64);

    while let Ok((stream, _)) = listener.accept().await {
        let mut ws_stream = ServerBuilder::new().accept(stream).await?;

        let chords_tx = chords_tx.clone();
        tokio::spawn(async move {
            // Receive identification message from client
            let Some(Ok(id)) = ws_stream.next().await else {
                anyhow::bail!("Failed to ID");
            };

            if let Ok(text) = id.as_text() {
                if let Ok(PublisherMessage::IAmPublisher { id }) = serde_json::from_str(text) {
                    println!("Identified \"{id}\" as publisher");
                    handle_publisher(chords_tx, ws_stream).await?;
                } else if let Ok(ConsumerMessage::IAmConsumer { id }) = serde_json::from_str(text) {
                    println!("Identified \"{id}\" as consumer");
                    let chords_rx = chords_tx.subscribe();
                    handle_consumer(chords_rx, ws_stream).await?;
                }
            }
            Ok::<_, anyhow::Error>(())
        });
    }
    Ok(())
}

async fn handle_publisher(
    chords_sender: broadcast::Sender<(HashSet<Note>, Option<Chord>)>,
    mut stream: WebsocketStream<TcpStream>,
) -> anyhow::Result<()> {
    while let Some(Ok(msg)) = stream.next().await {
        if let Ok(text) = msg.as_text() {
            if let Ok(PublisherMessage::PublishChord(notes, chord)) = serde_json::from_str(text) {
                if let Err(c) = chords_sender.send((notes, chord)) {
                    warn!("Currently no subscribed consumers, dropping {:?}", c.0);
                }
            }
        }
    }
    Ok(())
}

async fn handle_consumer(
    mut chords_receiver: broadcast::Receiver<(HashSet<Note>, Option<Chord>)>,
    mut stream: WebsocketStream<TcpStream>,
) -> anyhow::Result<()> {
    while let Ok((notes, chord)) = chords_receiver.recv().await {
        stream
            .send(ConsumerMessage::ChordEvent(notes, chord).to_message())
            .await?;
    }
    Ok(())
}
