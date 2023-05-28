use std::{
    collections::HashSet,
    fs::File,
    io::BufReader,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use futures_util::SinkExt;
use klib::core::{chord::Chord, note::Note};
use morivar::{ConsumerMessage, PublisherMessage};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    sync::broadcast,
};
use tokio_websockets::{ServerBuilder, WebsocketStream};
use tracing::{info, warn};

use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::{
    rustls::{self, Certificate, PrivateKey},
    TlsAcceptor,
};

const DEFAULT_PATH_TO_CERT: &str = "certs/localhost.crt";
const DEFAULT_PATH_TO_KEY: &str = "certs/localhost.key";

fn load_certs(path: impl AsRef<Path>) -> anyhow::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|certs| certs.into_iter().map(Certificate).collect())
        .context("Failed to load local certificates")
}

fn load_keys(path: impl AsRef<Path>) -> anyhow::Result<Vec<PrivateKey>> {
    pkcs8_private_keys(&mut BufReader::new(File::open(path)?))
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid key"))
        .map(|keys| keys.into_iter().map(PrivateKey).collect())
        .context("Failed to load local keys")
}

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(short, long, default_value = "127.0.0.1:8000")]
    address: SocketAddr,

    #[command(subcommand)]
    mode: Option<SecurityMode>,

    #[arg(long, default_value_t = 64)]
    chords_channel_size: usize,
}

#[derive(Debug, Subcommand)]
#[group(required = true, multiple = true)]
enum SecurityMode {
    /// Use a certificate and key file for SSL-encrypted communication
    Secure {
        #[arg(short, long, default_value = DEFAULT_PATH_TO_CERT)]
        cert: PathBuf,

        #[arg(short, long, default_value = DEFAULT_PATH_TO_KEY)]
        key: PathBuf,
    },
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
                let ws = ServerBuilder::new()
                    .accept(stream)
                    .await
                    .context("Failed to accept websocket client")?;

                handle_client(ws, chords_tx).await?;
            } else {
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
