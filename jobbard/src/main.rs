#![feature(never_type)]
#![doc = include_str!("../README.md")]

use std::{fs::File, io::BufReader, path::PathBuf, sync::Arc, time::Duration};

use anyhow::Context;
use clap::{command, Parser};
use client_utils::{
    announce_as_publisher, announce_protocol_version, create_client, create_uri, create_watchdog,
    flatten,
};
use futures_util::SinkExt;
use klib::core::{
    chord::{Chord, Chordable},
    modifier::{Degree, Extension, Modifier},
    note,
};
use morivar::{PublisherToServer, ServerToPublisher, ToMessage};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    select,
};
use tokio_websockets::WebsocketStream;
use tracing::{info, warn};
use watchdog::{Expired, Signal};

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
    let secure = args.secure;
    let id = args.id;

    if let Some(path) = template {
        let song = simple_sequence();
        std::fs::write(path, serde_json::to_string_pretty(&song).unwrap()).unwrap();
        info!("Wrote template song, exiting");
        return Ok(());
    }

    let song: Vec<Chord> = serde_json::from_reader(BufReader::new(File::open(song)?))?;
    let song = Arc::new(song);

    let uri = create_uri(args.url, secure)?;

    loop {
        let id = id.clone();
        let uri = uri.clone();
        let song = Arc::clone(&song);
        let handle = tokio::spawn(async move {
            let song = song.iter().cycle();
            info!("Attempting to connect to server");
            let mut stream = create_client(&uri, secure).await?;

            let result = jobbard(&mut stream, &id, args.pingpong, &interval, song.clone()).await;
            if let Err(e) = stream
                .send(PublisherToServer::PublishSilence.to_message())
                .await
            {
                warn!("Failed to publish final silence: {e:?}");
            }
            if let Err(e) = stream.close(None, None).await {
                warn!("Failed to close the stream: {e:?}");
            }
            result
        });
        let error = tokio::try_join!(flatten(handle));
        warn!("Failed to handle connection: {error:?}");
        tokio::time::sleep(morivar::CLIENT_RECONNECT_DURATION).await;
    }
}

async fn jobbard<S>(
    stream: &mut WebsocketStream<S>,
    id: &str,
    pingpong: bool,
    interval: &Duration,
    mut song: impl Iterator<Item = &Chord>,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    announce_protocol_version(stream).await?;

    announce_as_publisher(id, stream).await?;

    let mut chord_interval = tokio::time::interval(*interval + Duration::from_millis(500));

    let (mut interval, watchdog, mut expiration) = create_watchdog().await?;

    loop {
        select! {
            msg = stream.next() => {
                let Some(Ok(msg)) = msg else {
                    anyhow::bail!("Error receiving message: {msg:?}");
                };
                let Ok(msg) = msg.as_text() else {
                   anyhow::bail!("Expected text message, got: {msg:?}");
                };
                let Ok(ServerToPublisher::Pong) = serde_json::from_str(msg) else {
                    anyhow::bail!("Expected Pong, got: {msg:?}");
                };
                watchdog.send(Signal::Stop).await.context("Failed to stop watchdog")?;
            }
            _p = chord_interval.tick() => {
                let chord = song.next().unwrap();
                info!("Sending chord {chord}");
                stream
                    .send(PublisherToServer::PublishChord(chord.clone()).to_message())
                    .await?;
                stream.send(PublisherToServer::PublishSilence.to_message()).await?;
            }
            _i = interval.tick(), if pingpong => {
                info!("Sending Ping!");
                watchdog.send(Signal::Reset).await?;
                stream.send(PublisherToServer::Ping.to_message()).await?;
            }
            e = &mut expiration, if pingpong => {
                let Expired = e.context("Failed to monitor watchdog")?;
                anyhow::bail!("Server failed to pong");
            }
        }
    }
}
