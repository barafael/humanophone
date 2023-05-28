use std::net::SocketAddr;

use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
use morivar::ConsumerMessage;
use native_tls::Certificate;
use tokio_websockets::ClientBuilder;
use tracing::{info, warn};

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(short, long, default_value = "127.0.0.1:8000")]
    address: SocketAddr,

    /// The id to report to Quinnipak
    #[arg(short, long, default_value = "I am Pehnt")]
    id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();

    let uri = Uri::builder()
        .scheme("wss")
        .authority(args.address.to_string())
        .path_and_query("/")
        .build()?;
    let bytes = std::fs::read("../quinnipak/certs/localhost.crt")?;
    let cert = Certificate::from_pem(&bytes)?;
    let connector = native_tls::TlsConnector::builder()
        .add_root_certificate(cert)
        .build()?;
    let connector = tokio_websockets::Connector::NativeTls(connector.into());

    let mut client = ClientBuilder::from_uri(uri)
        .connector(&connector)
        .connect()
        .await?;

    let announce = ConsumerMessage::IAmConsumer { id: args.id };
    client.send(announce.to_message()).await?;

    loop {
        let next = client.next().await;
        if let Some(Ok(msg)) = next {
            if let Ok(text) = msg.as_text() {
                if let Ok(ConsumerMessage::ChordEvent(notes, chord)) = serde_json::from_str(text) {
                    info!("{notes:?}");
                    if let Some(chord) = chord {
                        info!("{chord}");
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
