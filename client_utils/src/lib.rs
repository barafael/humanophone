#![doc = include_str!("../README.md")]

use std::time::Duration;

use anyhow::{anyhow, Context};
use futures_util::SinkExt;
use http::{uri::Authority, Uri};
use morivar::{ClientToServer, ToMessage};
use rand::{thread_rng, Rng};
use simple_tokio_watchdog::{Expired, Signal, Watchdog};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::Interval,
};
use tokio_native_tls::native_tls;
use tokio_websockets::{ClientBuilder, MaybeTlsStream, WebsocketStream};
use tracing::info;

pub fn create_uri(uri: Authority, secure: bool) -> Result<Uri, http::Error> {
    Uri::builder()
        .scheme(if secure { "wss" } else { "ws" })
        .authority(uri)
        .path_and_query("/")
        .build()
}

pub async fn create_client(
    uri: &Uri,
    secure: bool,
) -> anyhow::Result<WebsocketStream<MaybeTlsStream<TcpStream>>> {
    if secure {
        let connector = native_tls::TlsConnector::builder().build()?;
        let connector = tokio_websockets::Connector::NativeTls(connector.into());

        ClientBuilder::from_uri(uri.clone())
            .connector(&connector)
            .connect()
            .await
    } else {
        ClientBuilder::from_uri(uri.clone()).connect().await
    }
    .context("Failed to connect to server")
}

pub async fn announce_protocol_version<S>(stream: &mut WebsocketStream<S>) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    info!("Announcing protocol version");
    let version = ClientToServer::ProtocolVersion(morivar::PROTOCOL_VERSION);
    stream
        .send(version.to_message())
        .await
        .context("Failed to send protocol version")
}

pub async fn announce_as_consumer<S>(
    id: impl ToString,
    stream: &mut WebsocketStream<S>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    info!("Announcing as consumer");
    let announce = ClientToServer::IAmConsumer { id: id.to_string() };
    stream
        .send(announce.to_message())
        .await
        .context("Failed to send consumer announcement")
}

pub async fn announce_as_publisher<S>(
    id: impl ToString,
    stream: &mut WebsocketStream<S>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    info!("Announcing as publisher");
    let announce = ClientToServer::IAmPublisher { id: id.to_string() };
    stream
        .send(announce.to_message())
        .await
        .context("Failed to send publisher announcement")
}

pub async fn create_watchdog(
) -> anyhow::Result<(Interval, mpsc::Sender<Signal>, oneshot::Receiver<Expired>)> {
    let interval = tokio::time::interval(morivar::PING_INTERVAL);

    let (watchdog, expiration) = Watchdog::with_timeout(morivar::PING_TO_PONG_ALLOWED_DELAY).run();
    watchdog
        .send(Signal::Stop)
        .await
        .expect("It's the first message");

    Ok((interval, watchdog, expiration))
}

pub async fn flatten<T>(handle: JoinHandle<anyhow::Result<T>>) -> anyhow::Result<T> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(anyhow!(err)),
    }
}

pub fn jittering_retry_duration() -> Duration {
    morivar::CLIENT_RECONNECT_DURATION + jitter_duration()
}

fn jitter_duration() -> Duration {
    Duration::from_millis(thread_rng().gen_range(0..=250))
}
