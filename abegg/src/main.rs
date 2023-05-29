#![doc = include_str!("../README.md")]

use std::net::SocketAddr;

use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
use klib::core::base::Playable;
use morivar::ConsumerMessage;
use native_tls::Certificate;
use pitches::Pitches;
use tokio_websockets::ClientBuilder;
use tracing::warn;

use jun::SecurityMode;

mod pitches;

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(short, long, default_value = "0.0.0.0:8000")]
    address: SocketAddr,

    #[command(subcommand)]
    mode: Option<SecurityMode>,

    /// The id to report to Quinnipak
    #[arg(short, long, default_value = "I am Abegg")]
    id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();

    let scheme = if matches!(args.mode, Some(SecurityMode::Secure { .. })) {
        "wss"
    } else {
        "ws"
    };
    let uri = Uri::builder()
        .scheme(scheme)
        .authority(args.address.to_string())
        .path_and_query("/")
        .build()?;

    let mut client = if let Some(SecurityMode::Secure { cert, .. }) = args.mode {
        let bytes = std::fs::read(cert)?;
        let cert = Certificate::from_pem(&bytes)?;
        let connector = native_tls::TlsConnector::builder()
            .add_root_certificate(cert)
            .build()?;
        let connector = tokio_websockets::Connector::NativeTls(connector.into());

        ClientBuilder::from_uri(uri)
            .connector(&connector)
            .connect()
            .await?
    } else {
        ClientBuilder::from_uri(uri).connect().await?
    };

    let announce = ConsumerMessage::IAmConsumer { id: args.id };
    client.send(announce.to_message()).await?;

    let mut handle = None;
    loop {
        let next = client.next().await;
        if let Some(Ok(msg)) = next {
            if let Ok(text) = msg.as_text() {
                if let Ok(ConsumerMessage::ChordEvent(notes, chord)) = serde_json::from_str(text) {
                    if let Some(chord) = chord {
                        let ph = chord.play(0.0, 5.0, 0.5)?;
                        let _ = handle.insert(ph);
                    } else {
                        let ph = Pitches::from(notes).play(0.0, 5.0, 0.5)?;
                        let _ = handle.insert(ph);
                    }
                }
            } else {
                warn!("Stopping receive");
                break;
            }
        } else {
            warn!("Breaking on client message: {next:?}");
            break;
        }
    }

    client.close(None, None).await?;
    Ok(())
}
