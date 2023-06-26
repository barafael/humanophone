#![doc = include_str!("../README.md")]

use std::{collections::HashSet, time::Duration};

use klib::core::{chord::Chord, note::Note};
use serde::{Deserialize, Serialize};
#[cfg(feature = "message")]
use tokio_websockets::Message;

pub const PING_INTERVAL: Duration = Duration::from_secs(10);
pub const PING_AWAIT_INTERVAL: Duration = Duration::from_secs(15);
pub const PING_TO_PONG_ALLOWED_DELAY: Duration = Duration::from_secs(5);

#[cfg(feature = "cli")]
pub mod cli;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublisherMessage {
    IAmPublisher { id: String },
    Protocol { version: u32 },
    PublishChord(Chord),
    PublishPitches(HashSet<Note>),
    Silence,
    NowAreYou,
    InvalidMessage(String),
    Ping,
    Pong,
}

#[cfg(feature = "message")]
impl PublisherMessage {
    #[must_use]
    pub fn to_message(self) -> Message {
        let message = serde_json::to_string_pretty(&self).expect("Serialization failed");
        Message::text(message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsumerMessage {
    IAmConsumer { id: String },
    Protocol { version: u32 },
    ChordEvent(Chord),
    PitchesEvent(HashSet<Note>),
    Silence,
    Ping,
    Pong,
}

#[cfg(feature = "message")]
impl ConsumerMessage {
    #[must_use]
    pub fn to_message(self) -> Message {
        let message = serde_json::to_string_pretty(&self).expect("Serialization failed");
        Message::text(message)
    }
}

#[cfg(test)]
mod test {
    use klib::core::{chord::Chordable, named_pitch::NamedPitch, note, octave::Octave};

    use super::*;

    #[test]
    fn serializes_announce() {
        let message = ConsumerMessage::IAmConsumer {
            id: "Hello there".to_string(),
        };
        dbg!(serde_json::to_string_pretty(&message).unwrap());
    }

    #[test]
    fn serializes_piano_announce() {
        let message = PublisherMessage::IAmPublisher {
            id: "Hello there".to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&message).unwrap());
    }

    #[test]
    fn serializes_piano_chord() {
        let chord = Chord::new(note::AFlat).sus4().seven().add13();
        let message = PublisherMessage::PublishChord(chord);
        println!("{:?}", serde_json::to_string_pretty(&message).unwrap());
    }

    #[test]
    fn serializes_piano_pitches() {
        let chord = vec![
            Note::new(NamedPitch::A, Octave::Four),
            Note::new(NamedPitch::C, Octave::Five),
        ]
        .into_iter()
        .collect();
        let message = PublisherMessage::PublishPitches(chord);
        println!("{:?}", serde_json::to_string(&message).unwrap());
    }
}
