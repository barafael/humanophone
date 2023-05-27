use std::{collections::HashSet, net::SocketAddr};

use clap::Parser;
use futures_util::SinkExt;
use klib::core::{chord::Chord, note::Note};
use morivar::{ConsumerMessage, PublisherMessage};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast,
};
use tokio_websockets::{ServerBuilder, WebsocketStream};
use tracing::{info, warn};

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(short, long, default_value = "0.0.0.0:8000")]
    address: SocketAddr,

    #[arg(short, long, default_value_t = 64)]
    chords_channel_size: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();

    let (chords_tx, _) = broadcast::channel(args.chords_channel_size);

    let listener = TcpListener::bind(args.address).await?;

    while let Ok((stream, _)) = listener.accept().await {
        let mut ws_stream = match ServerBuilder::new().accept(stream).await {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to accept websocket client: {e}");
                continue;
            }
        };

        let chords_tx = chords_tx.clone();
        tokio::spawn(async move {
            // Receive identification message from client
            let Some(Ok(identification)) = ws_stream.next().await else {
                anyhow::bail!("Failed to ID");
            };

            if let Ok(text) = identification.as_text() {
                if let Ok(PublisherMessage::IAmPublisher { id }) = serde_json::from_str(text) {
                    info!("Identified \"{id}\" as publisher");
                    handle_publisher(chords_tx, ws_stream).await?;
                } else if let Ok(ConsumerMessage::IAmConsumer { id }) = serde_json::from_str(text) {
                    info!("Identified \"{id}\" as consumer");
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
