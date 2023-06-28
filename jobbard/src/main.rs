#![doc = include_str!("../README.md")]

use std::{fs::File, io::BufReader, path::PathBuf, time::Duration};

use anyhow::{Context, Ok};
use clap::{command, Parser};
use futures_util::SinkExt;
use http::Uri;
use klib::core::{
    chord::{Chord, Chordable},
    modifier::{Degree, Extension, Modifier},
    note,
};
use morivar::PublisherMessage;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    select,
};
use tokio_native_tls::native_tls;
use tokio_websockets::{ClientBuilder, WebsocketStream};
use tracing::{info, warn};
use watchdog::{Expired, Signal, Watchdog};

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[command(flatten)]
    args: morivar::cli::ClientArguments<{ env!("CARGO_BIN_NAME") }>,

    /// The input file containing chords
    #[arg(long, default_value = "song.json")]
    song: PathBuf,

    /// A path for generating a simple song template file with 4 chords in it
    #[arg(short, long)]
    template: Option<PathBuf>,

    /// The interval to play new chords at
    #[arg(long, default_value_t = Duration::from_secs(5).into())]
    interval: humantime::Duration,
}

fn simple_sequence() -> [Chord; 4] {
    let gm9 = Chord::new(note::G).minor().seven().add9().add11();
    let c9 = Chord::new(note::C)
        .dominant(Degree::Seven)
        .with_modifier(Modifier::Flat9)
        .add13();
    let f69 = Chord::new(note::F).add_six().add_nine();
    let ab13 = Chord::new(note::D)
        .dominant7()
        .with_extension(Extension::Flat13)
        .with_modifier(Modifier::Sharp9);
    [gm9, c9, f69, ab13]
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();
    let song = args.song;
    let template = args.template;
    let interval = args.interval;
    let args = args.args;

    if let Some(path) = template {
        let song = simple_sequence();
        std::fs::write(path, serde_json::to_string_pretty(&song).unwrap()).unwrap();
        info!("Wrote template song, exiting");
        return Ok(());
    }

    let song: Vec<Chord> = serde_json::from_reader(BufReader::new(File::open(song)?))?;
    let song = song.iter().cycle();

    let uri = Uri::builder()
        .scheme(if args.secure { "wss" } else { "ws" })
        .authority(args.url)
        .path_and_query("/")
        .build()?;

    loop {
        info!("Attempting to connect to server");
        let stream = if args.secure {
            let connector = native_tls::TlsConnector::builder().build()?;
            let connector = tokio_websockets::Connector::NativeTls(connector.into());

            ClientBuilder::from_uri(uri.clone())
                .connector(&connector)
                .connect()
                .await?
        } else {
            ClientBuilder::from_uri(uri.clone()).connect().await?
        };

        if let Err(e) =
            handle_connection(stream, &args.id, args.pingpong, &interval, song.clone()).await
        {
            warn!("Failed to handle connection: {e:?}");
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

async fn handle_connection<S>(
    mut stream: WebsocketStream<S>,
    id: &str,
    pingpong: bool,
    interval: &Duration,
    mut song: impl Iterator<Item = &Chord>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let version = PublisherMessage::ProtocolVersion(morivar::PROTOCOL_VERSION);
    stream.send(version.to_message()).await?;

    let announce = PublisherMessage::IAmPublisher { id: id.to_string() };
    stream.send(announce.to_message()).await?;

    let mut chord_interval = tokio::time::interval(*interval + Duration::from_millis(500));

    let mut interval = tokio::time::interval(morivar::PING_INTERVAL);

    let (watchdog, mut expiration) =
        Watchdog::with_timeout(morivar::PING_TO_PONG_ALLOWED_DELAY).run();
    watchdog
        .send(Signal::Stop)
        .await
        .expect("It's the first message");

    let error = loop {
        select! {
            _p = chord_interval.tick() => {
                let chord = song.next().unwrap();
                info!("Sending chord {chord}");
                stream
                    .send(PublisherMessage::PublishChord(chord.clone()).to_message())
                    .await?;
                stream.send(PublisherMessage::Silence.to_message()).await?;
            }
            _i = interval.tick(), if pingpong => {
                info!("Sending Ping!");
                watchdog.send(Signal::Reset).await?;
                stream.send(PublisherMessage::Ping.to_message()).await?;
            }
            e = &mut expiration, if pingpong => {
                let Expired = e.context("Failed to monitor watchdog")?;
                break anyhow::anyhow!("Server failed to pong");
            }
        }
    };

    stream.send(PublisherMessage::Silence.to_message()).await?;
    stream
        .close(None, None)
        .await
        .context("Failed to close websocket client")?;
    Err(error)
}
