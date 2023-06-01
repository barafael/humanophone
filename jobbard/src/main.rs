#![doc = include_str!("../README.md")]

use std::{fs::File, io::BufReader, path::PathBuf, time::Duration};

use anyhow::{Context, Ok};
use clap::{command, Parser, ValueHint};
use futures_util::SinkExt;
use http::{uri::Authority, Uri};
use klib::core::{
    chord::{Chord, Chordable},
    modifier::Degree,
    note,
};
use morivar::PublisherMessage;
use tokio_native_tls::native_tls;
use tokio_websockets::ClientBuilder;
use tracing::info;

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[arg(short, long, value_hint = ValueHint::Url, default_value = "0.0.0.0:8000")]
    url: Authority,

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

    #[arg(short, long, default_value_t = false)]
    secure: bool,
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

    let uri = Uri::builder()
        .scheme(if args.secure { "wss" } else { "ws" })
        .authority(args.url)
        .path_and_query("/")
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
