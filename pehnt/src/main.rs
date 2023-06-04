#![doc = include_str!("../README.md")]

use clap::{command, Parser, ValueHint};
use futures_util::SinkExt;
use http::{uri::Authority, Uri};
use morivar::ConsumerMessage;
use tokio::select;
use tokio_websockets::ClientBuilder;
use tracing::{info, warn};
use watchdog::{Reset, Watchdog};

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(short, long, value_hint = ValueHint::Url, default_value = "0.0.0.0:8000")]
    url: Authority,

    /// The id to report to Quinnipak
    #[arg(short, long, default_value = "Pehnt")]
    id: String,

    #[arg(short, long, default_value_t = false)]
    secure: bool,

    #[arg(short, long, default_value_t = false)]
    pingpong: bool,
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

    let mut stream = if args.secure {
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
    stream.send(announce.to_message()).await?;

    let mut interval = tokio::time::interval(morivar::PING_INTERVAL);

    let (resetter, mut expired) =
        Watchdog::with_timeout(morivar::PING_TO_PONG_ALLOWED_DELAY).spawn();
    loop {
        select! {
            next = stream.next() => {
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
                            Ok(ConsumerMessage::Pong) => {
                                resetter.send(Reset::Stop).await?;
                                info!("Received Pong!");
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
            _i = interval.tick(), if args.pingpong => {
                info!("Sending Ping!");
                resetter.send(Reset::Start).await?;
                stream.send(ConsumerMessage::Ping.to_message()).await?;
            }
            e = &mut expired, if args.pingpong => {
                match e {
                    Ok(_expired) => {
                        anyhow::bail!("Server failed to pong")
                    }
                    Err(e) => anyhow::bail!("Failed to monitor watchdog: {e:?}")
                }
            }
        }
    }

    stream.close(None, None).await?;
    Ok(())
}
