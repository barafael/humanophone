#![doc = include_str!("../README.md")]

use std::{net::SocketAddr, time::Duration};

use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
use klib::core::{base::Playable, named_pitch::NamedPitch, note::Note, octave::Octave};
use morivar::ConsumerMessage;
use native_tls::Certificate;
use once_cell::sync::Lazy;
use pitches::Pitches;
use tokio_websockets::ClientBuilder;
use tone::Tone;
use tracing::warn;

use jun::SecurityMode;

mod pitches;
mod tone;

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

    #[arg(long, default_value_t = false)]
    jingle: bool,
}

static ABEGG: Lazy<[(Note, f32, f32); 5]> = Lazy::new(|| {
    [
        (Note::new(NamedPitch::A, Octave::Four), 0.1, 0.05),
        (Note::new(NamedPitch::B, Octave::Four), 0.07, 0.2),
        (Note::new(NamedPitch::E, Octave::Five), 0.1, 0.05),
        (Note::new(NamedPitch::G, Octave::Five), 0.1, 0.05),
        (Note::new(NamedPitch::G, Octave::Five), 0.1, 0.05),
    ]
});

fn jingle(events: &[(Note, f32, f32)]) -> anyhow::Result<()> {
    for (note, length, pause) in events {
        let _handle = Tone::from(*note).play(0.0, *length, 0.01)?;
        std::thread::sleep(Duration::from_secs_f32(*length));
        std::thread::sleep(Duration::from_secs_f32(*pause));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();

    if args.jingle {
        jingle(&*ABEGG)?;
    }

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
