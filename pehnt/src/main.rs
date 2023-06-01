#![doc = include_str!("../README.md")]

use clap::{command, Parser, ValueHint};
use futures_util::SinkExt;
use http::{uri::Authority, Uri};
use morivar::ConsumerMessage;
use tokio_websockets::ClientBuilder;
use tracing::{info, warn};

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(short, long, value_hint = ValueHint::Url, default_value = "0.0.0.0:8000")]
    url: Authority,

    /// The id to report to Quinnipak
    #[arg(short, long, default_value = "I am Pehnt")]
    id: String,

    #[arg(short, long, default_value_t = false)]
    secure: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();

    let uri = Uri::builder()
        .scheme(if args.secure { "wss" } else { "ws" })
        .authority(args.url)
        .path_and_query("")
        .build()?;

    let mut client = if args.secure {
        let connector = native_tls::TlsConnector::builder().build()?;
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

    loop {
        let next = client.next().await;
        if let Some(Ok(msg)) = next {
            if let Ok(text) = msg.as_text() {
                match serde_json::from_str(text) {
                    Ok(ConsumerMessage::ChordEvent(chord)) => {
                        info!("Chord: {chord}");
                    }
                    Ok(ConsumerMessage::PitchesEvent(pitches)) => {
                        info!("Pitches: {pitches:?}");
                    }
                    Ok(ConsumerMessage::Silence) => {
                        info!("SILENCE!!!");
                    }
                    m => {
                        warn!("Unhandled consumer message: {m:?}");
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
