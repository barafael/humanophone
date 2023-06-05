#![doc = include_str!("../README.md")]

use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use clap::Parser;
use morivar::{ConsumerMessage, PublisherMessage};
use secure::{load_certs, load_keys, SecurityMode};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    sync::broadcast,
};
use tokio_rustls::TlsAcceptor;
use tokio_websockets::{ServerBuilder, WebsocketStream};
use tracing::{info, warn};

use crate::{consumer::handle_consumer, publisher::handle_publisher};

mod consumer;
mod publisher;
mod secure;

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    /// The address to bind on
    #[arg(short, long, default_value = "0.0.0.0:8000")]
    address: SocketAddr,

    /// The security mode
    #[command(subcommand)]
    mode: Option<SecurityMode>,

    /// The channel size for the chord broadcast
    #[arg(long, default_value_t = 64)]
    chords_channel_size: usize,

    /// Whether to monitor consumers for pings
    #[arg(long, default_value_t = false)]
    pingpong: bool,
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
            if let Err(e) = handle_connection(stream, chords_tx, acceptor, args.pingpong).await {
                warn!("Error while handling connection: {e:?}")
            }
        });
    }
    Ok(())
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    chords_tx: broadcast::Sender<ConsumerMessage>,
    acceptor: Option<TlsAcceptor>,
    pingpong: bool,
) -> anyhow::Result<()> {
    if let Some(acceptor) = acceptor {
        let stream = acceptor.accept(stream).await?;
        // The type of `wss` is `WebsocketStream<TlsStream<TcpStream>>`
        let wss = ServerBuilder::new()
            .accept(stream)
            .await
            .context("Failed to accept secured websocket client")?;

        handle_client(wss, chords_tx, pingpong).await?;
    } else {
        // The type of `ws` is `WebsocketStream<TcpStream>`
        let ws = ServerBuilder::new()
            .accept(stream)
            .await
            .context("Failed to accept websocket client")?;
        handle_client(ws, chords_tx, pingpong).await?;
    }
    anyhow::Ok(())
}

async fn handle_client<S>(
    mut stream: WebsocketStream<S>,
    chords_sender: broadcast::Sender<ConsumerMessage>,
    pingpong: bool,
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
            handle_publisher(chords_sender, stream, pingpong).await?;
        } else if let Ok(ConsumerMessage::IAmConsumer { id }) = serde_json::from_str(text) {
            info!("Identified \"{id}\" as consumer");
            let chords_rx = chords_sender.subscribe();
            handle_consumer(chords_rx, stream, pingpong).await?;
        }
    }
    Ok(())
}
