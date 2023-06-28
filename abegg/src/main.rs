#![doc = include_str!("../README.md")]

use std::time::Duration;

use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
use klib::core::{base::Playable, named_pitch::NamedPitch, note::Note, octave::Octave};
use morivar::ConsumerMessage;
use once_cell::sync::Lazy;
use pitches::Pitches;
use tokio_websockets::ClientBuilder;
use tracing::warn;

mod pitches;

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[command(flatten)]
    args: morivar::cli::ClientArguments<{ env!("CARGO_BIN_NAME") }>,

    /// Whether to play the ABEGG jingle
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
        let _handle = note.play(
            Duration::ZERO,
            Duration::from_secs_f32(*length),
            Duration::from_millis(10),
        )?;
        std::thread::sleep(Duration::from_secs_f32(*length));
        std::thread::sleep(Duration::from_secs_f32(*pause));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();
    let play_jingle = args.jingle;
    let args = args.args;

    if play_jingle {
        jingle(&*ABEGG)?;
    }

    let uri = Uri::builder()
        .scheme(if args.secure { "wss" } else { "ws" })
        .authority(args.url)
        .path_and_query("/")
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

    let version = ConsumerMessage::ProtocolVersion(morivar::PROTOCOL_VERSION);
    stream.send(version.to_message()).await?;

    let announce = ConsumerMessage::IAmConsumer { id: args.id };
    stream.send(announce.to_message()).await?;

    let mut handle = None;
    loop {
        let next = stream.next().await;
        if let Some(Ok(msg)) = next {
            if let Ok(text) = msg.as_text() {
                match serde_json::from_str(text) {
                    Ok(ConsumerMessage::ChordEvent(chord)) => {
                        let ph = chord.play(
                            Duration::ZERO,
                            Duration::from_secs(5),
                            Duration::from_millis(500),
                        )?;
                        let _ = handle.insert(ph);
                    }
                    Ok(ConsumerMessage::PitchesEvent(pitches)) => {
                        let ph = Pitches::from(pitches).play(
                            Duration::ZERO,
                            Duration::from_secs(5),
                            Duration::from_millis(500),
                        )?;
                        let _ = handle.insert(ph);
                    }
                    Ok(ConsumerMessage::Silence) => {
                        handle = None;
                    }
                    e => {
                        warn!("Unhandled event: {e:?}");
                        break;
                    }
                }
            }
        } else {
            warn!("Breaking on client message: {next:?}");
            break;
        }
    }

    stream.close(None, None).await?;
    Ok(())
}
