#![doc = include_str!("../README.md")]

use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use clap::Parser;
use futures_util::SinkExt;
use morivar::{ConsumerMessage, PublisherMessage};
use secure::{load_certs, load_keys, SecurityMode};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    sync::broadcast,
};
use tokio_rustls::TlsAcceptor;
use tokio_websockets::{Message, ServerBuilder, WebsocketStream};
use tracing::{info, warn};

mod secure;

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
    chords_sender: broadcast::Sender<ConsumerMessage>,
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
    chords_sender: broadcast::Sender<ConsumerMessage>,
    mut stream: WebsocketStream<S>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    while let Some(Ok(msg)) = stream.next().await {
        if let Ok(text) = msg.as_text() {
            match serde_json::from_str(text) {
                Ok(PublisherMessage::PublishChord(chord)) => {
                    println!("{:?}", chord);
                    if let Err(c) = chords_sender.send(ConsumerMessage::ChordEvent(chord)) {
                        warn!("Currently no subscribed consumers, dropping {:?}", c.0);
                    }
                }
                Ok(PublisherMessage::PublishPitches(pitches)) => {
                    if let Err(c) = chords_sender.send(ConsumerMessage::PitchesEvent(pitches)) {
                        warn!("Currently no subscribed consumers, dropping {:?}", c.0);
                    }
                }
                Ok(PublisherMessage::Silence) => {
                    if let Err(c) = chords_sender.send(ConsumerMessage::Silence) {
                        warn!("Currently no subscribed consumers, dropping {:?}", c.0);
                    }
                }
                e => {
                    warn!("Unhandled publication: {e:?}");
                    chords_sender.send(ConsumerMessage::Silence)?;
                    break;
                }
            }
        }
    }
    Ok(())
}

async fn handle_consumer<S>(
    mut chords_receiver: broadcast::Receiver<ConsumerMessage>,
    mut stream: WebsocketStream<S>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    loop {
        tokio::select! {
            event = chords_receiver.recv() => {
                let m = event.context("Failed to receive message on internal chord bus")?;
                stream.send(m.to_message()).await?;
            }
            item = stream.next() => {
                match item {
                    Some(m) => {
                        let m = m.context("Error on websocket client connection")?;
                        warn!("Client sending not allowed. Client sent {m:?}");
                        stream.send(Message::text("A client shall not send after identifying.".to_string())).await?;
                        break;
                    }
                    None => {
                        info!("Stream ended!");
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
