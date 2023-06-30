#![doc = include_str!("../README.md")]

use anyhow::Context;
use morivar::PublisherToServer;
use morivar::PROTOCOL_VERSION;
use morivar::{ConsumerToServer, ServerToConsumer};
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::sync::broadcast;
use tokio_rustls::TlsAcceptor;
use tokio_websockets::Message;
use tokio_websockets::ServerBuilder;
use tokio_websockets::WebsocketStream;
use tracing::info;

pub mod cli;
mod consumer;
mod publisher;
pub mod secure;

pub async fn handle_connection<Stream>(
    stream: Stream,
    chords_tx: broadcast::Sender<ServerToConsumer>,
    acceptor: Option<TlsAcceptor>,
    pingpong: bool,
) -> anyhow::Result<()>
where
    Stream: AsyncRead + AsyncWrite + Unpin,
{
    if let Some(acceptor) = acceptor {
        info!("Accepting encrypted connection");
        let stream = acceptor.accept(stream).await?;
        // The type of `wss` is `WebsocketStream<TlsStream<TcpStream>>`
        let wss = ServerBuilder::new()
            .accept(stream)
            .await
            .context("Failed to accept secured websocket client")?;
        handle_client(wss, chords_tx, pingpong).await?;
    } else {
        info!("Accepting connection");
        // The type of `ws` is `WebsocketStream<TcpStream>`
        let ws = ServerBuilder::new()
            .accept(stream)
            .await
            .context("Failed to accept websocket client")?;
        handle_client(ws, chords_tx, pingpong).await?;
    }
    anyhow::Ok(())
}

pub async fn handle_client<T>(
    mut stream: WebsocketStream<T>,
    chords_sender: broadcast::Sender<ServerToConsumer>,
    pingpong: bool,
) -> anyhow::Result<()>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    info!("Expecting protocol version message from client");
    let Some(Ok(version)) = stream.next().await else {
        anyhow::bail!("Failed to get protocol version message");
    };
    let version = determine_protocol_version(&version).context("Protocol error")?;

    anyhow::ensure!(version == PROTOCOL_VERSION, "Protocol version mismatch");

    info!("Expecting identification message from client");
    let Some(Ok(identification)) = stream.next().await else {
        anyhow::bail!("Failed to ID");
    };
    let Ok(text) = identification.as_text() else {
        anyhow::bail!("Protocol error, second message wasn't a text message: {identification:?}");
    };
    if let Ok(PublisherToServer::IAmPublisher { id }) = serde_json::from_str(text) {
        info!("Identified \"{id}\" as publisher");
        publisher::run(chords_sender, stream, pingpong).await?;
    } else if let Ok(ConsumerToServer::IAmConsumer { id }) = serde_json::from_str(text) {
        info!("Identified \"{id}\" as consumer");
        let chords_rx = chords_sender.subscribe();
        consumer::run(chords_rx, stream, pingpong).await?;
    } else {
        anyhow::bail!("Protocol error, client identification failed: {text}");
    }
    Ok(())
}

fn determine_protocol_version(version: &Message) -> anyhow::Result<u32> {
    let Ok(text) = version.as_text() else {
        anyhow::bail!("initial message wasn't a text message: {version:?}");
    };
    if let Ok(PublisherToServer::ProtocolVersion(version)) = serde_json::from_str(text) {
        info!("Publisher client with protocol version {version}");
        Ok(version)
    } else if let Ok(ConsumerToServer::ProtocolVersion(version)) = serde_json::from_str(text) {
        info!("Consumer client with protocol version {version}");
        Ok(version)
    } else {
        anyhow::bail!("version decoding failed: {text}");
    }
}
