#![doc = include_str!("../README.md")]

use std::{fs::File, io::BufReader, net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::{Context, Ok};
use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
use klib::core::{
    chord::{Chord, Chordable},
    modifier::Degree,
    note,
};
use morivar::PublisherMessage;
use tokio_native_tls::native_tls::{self, Certificate};
use tokio_websockets::ClientBuilder;
use tracing::info;

use jun::SecurityMode;

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(short, long, default_value = "0.0.0.0:8000")]
    address: SocketAddr,

    /// The id to report to Quinnipak
    #[arg(short, long, default_value = "I am Jobbard")]
    id: String,

    /// The input file containing chords
    #[arg(short, long, default_value = "song.json")]
    song: PathBuf,

    /// A path for generating a simple song template file with 3 chords in it
    #[arg(short, long)]
    template: Option<PathBuf>,

    /// The interval to play new chords at
    #[arg(long, default_value_t = Duration::from_secs(5).into())]
    interval: humantime::Duration,

    #[command(subcommand)]
    mode: Option<SecurityMode>,
}

fn simple_sequence() -> [Chord; 3] {
    let gm9 = Chord::new(note::G).minor().seven().add9().add11();
    let c9 = Chord::new(note::C).dominant(Degree::Nine).seven();
    let f69 = Chord::new(note::F).add_six().add_nine();
    [gm9, c9, f69]
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();

    if let Some(path) = args.template {
        let song = simple_sequence();
        std::fs::write(path, serde_json::to_string_pretty(&song).unwrap()).unwrap();
        info!("Wrote template song, exiting");
        return Ok(());
    }

    let song: Vec<Chord> = serde_json::from_reader(BufReader::new(File::open(args.song)?))?;
    let song = song.iter().cycle();

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

    let announce = PublisherMessage::IAmPublisher { id: args.id };
    client.send(announce.to_message()).await?;

    for chord in song {
        info!("Sending chord {chord}");
        client
            .send(PublisherMessage::PublishChord(chord.clone()).to_message())
            .await?;
        tokio::time::sleep(args.interval.into()).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        client.send(PublisherMessage::Silence.to_message()).await?;
    }

    client.send(PublisherMessage::Silence.to_message()).await?;
    client
        .close(None, None)
        .await
        .context("Failed to close websocket client")
}
