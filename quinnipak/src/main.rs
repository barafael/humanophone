use std::{collections::HashSet, net::SocketAddr, sync::Arc};

use anyhow::Context;
use clap::Parser;
use futures_util::SinkExt;
use jun::{load_certs, load_keys, SecurityMode};
use klib::core::{chord::Chord, note::Note};
use morivar::{ConsumerMessage, PublisherMessage};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    sync::broadcast,
};
use tokio_rustls::TlsAcceptor;
use tokio_websockets::{ServerBuilder, WebsocketStream};
use tracing::{info, warn};

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(short, long, default_value = "0.0.0.0:8000")]
    address: SocketAddr,

    #[command(subcommand)]
    mode: Option<SecurityMode>,

    #[arg(long, default_value_t = 64)]
    chords_channel_size: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();

    let (chords_tx, _) = broadcast::channel(args.chords_channel_size);

    let listener = TcpListener::bind(args.address).await?;

    let acceptor = if let Some(SecurityMode::Secure { cert, key }) = args.mode {
        let certs = load_certs(cert)?;
        let mut keys = load_keys(key)?;

        let config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, keys.remove(0))
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        Some(TlsAcceptor::from(Arc::new(config)))
    } else {
        None
    };

    while let Ok((stream, _)) = listener.accept().await {
        let chords_tx = chords_tx.clone();
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            if let Some(acceptor) = acceptor {
                let stream = acceptor.accept(stream).await?;
                // The type of `ws` is `WebsocketStream<TlsStream<TcpStream>>`
                let ws = ServerBuilder::new()
                    .accept(stream)
                    .await
                    .context("Failed to accept websocket client")?;

                handle_client(ws, chords_tx).await?;
            } else {
                // The type of `ws` is `WebsocketStream<TcpStream>`
                let ws = ServerBuilder::new()
                    .accept(stream)
                    .await
                    .context("Failed to accept websocket client")?;
                handle_client(ws, chords_tx).await?;
            }
            anyhow::Ok(())
        });
    }
    Ok(())
}

async fn handle_client<S>(
    mut stream: WebsocketStream<S>,
    chords_sender: broadcast::Sender<(HashSet<Note>, Option<Chord>)>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // Receive identification message from client
    let Some(Ok(identification)) = stream.next().await else {
        anyhow::bail!("Failed to ID");
    };

    if let Ok(text) = identification.as_text() {
        if let Ok(PublisherMessage::IAmPublisher { id }) = serde_json::from_str(text) {
            info!("Identified \"{id}\" as publisher");
            handle_publisher(chords_sender, stream).await?;
        } else if let Ok(ConsumerMessage::IAmConsumer { id }) = serde_json::from_str(text) {
            info!("Identified \"{id}\" as consumer");
            let chords_rx = chords_sender.subscribe();
            handle_consumer(chords_rx, stream).await?;
        }
    }
    Ok(())
}

async fn handle_publisher<S>(
    chords_sender: broadcast::Sender<(HashSet<Note>, Option<Chord>)>,
    mut stream: WebsocketStream<S>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
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

async fn handle_consumer<S>(
    mut chords_receiver: broadcast::Receiver<(HashSet<Note>, Option<Chord>)>,
    mut stream: WebsocketStream<S>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    while let Ok((notes, chord)) = chords_receiver.recv().await {
        stream
            .send(ConsumerMessage::ChordEvent(notes, chord).to_message())
            .await?;
    }
    Ok(())
}
