use std::time::Duration;

use futures_util::SinkExt;
use http::Uri;
use humanophone::PublisherMessage;
use klib::core::{chord::Chord, named_pitch::NamedPitch, note::Note, octave::Octave};
use tokio_websockets::ClientBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let uri = Uri::from_static("ws://127.0.0.1:3000");
    let mut client = ClientBuilder::from_uri(uri).connect().await?;

    let a = Chord::new(Note::new(NamedPitch::A, Octave::default()));
    let d = Chord::new(Note::new(NamedPitch::D, Octave::default()));
    let e = Chord::new(Note::new(NamedPitch::E, Octave::default()));
    let sequence = [a.clone(), a, d, e];
    let mut sequence = sequence.into_iter().cycle();

    let announce = PublisherMessage::IAmPublisher {
        id: "I am a piano".to_string(),
    };
    client.send(announce.to_message()).await?;

    for _ in 0..10 {
        let chord = PublisherMessage::PublishChord(sequence.next().unwrap()).to_message();
        client.send(chord).await?;
        std::thread::sleep(Duration::from_secs(5));
    }

    client.close(None, None).await?;
    Ok(())
}
