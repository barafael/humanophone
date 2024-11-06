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
use proptest::{
    arbitrary::arbitrary,
    prelude::{any, Arbitrary},
    strategy::{NewTree, Strategy},
    test_runner::{Config, TestRunner},
};
use tokio_native_tls::native_tls;
use tokio_websockets::ClientBuilder;
use tracing::info;

#[derive(Debug, Parser)]
#[command(author, version)]
struct Arguments {
    #[command(flatten)]
    args: morivar::cli::ClientArguments,

    /// The interval to play new chords at
    #[arg(long, default_value_t = Duration::from_secs(5).into())]
    interval: humantime::Duration,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Arguments::parse();
    let interval = args.interval;
    let args = args.args;

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

    let announce = PublisherMessage::IAmPublisher {
        id: args.id.unwrap_or("Slinger".to_string()),
    };
    client.send(announce.to_message()).await?;
    
    let chord: <Chord as Arbitrary>::Strategy = any::<Chord>();

    let mut runner = TestRunner::new(Config::default());

    let tree = chord.new_tree(&mut runner).unwrap();

    //client
    //.send(PublisherMessage::PublishChord(chord.clone()).to_message())
    //.await?;
    tokio::time::sleep(interval.into()).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    client.send(PublisherMessage::Silence.to_message()).await?;

    client.send(PublisherMessage::Silence.to_message()).await?;
    client
        .close(None, None)
        .await
        .context("Failed to close websocket client")
}
