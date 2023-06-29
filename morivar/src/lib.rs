#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![doc = include_str!("../README.md")]

use std::{collections::HashSet, time::Duration};

use klib::core::{chord::Chord, note::Note};
use serde::{Deserialize, Serialize};

#[cfg(feature = "message")]
pub mod to_message;

#[cfg(feature = "message")]
pub use to_message::ToMessage;

pub const PING_INTERVAL: Duration = Duration::from_secs(10);
pub const PING_AWAIT_INTERVAL: Duration = Duration::from_secs(15);
pub const PING_TO_PONG_ALLOWED_DELAY: Duration = Duration::from_secs(5);

#[cfg(feature = "cli")]
pub mod cli;

pub const PROTOCOL_VERSION: u32 = 1;

pub const CLIENT_RECONNECT_DURATION: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublisherToServer {
    IAmPublisher {
        id: String,
    },
    #[serde(rename = "PublisherProtocolVersion")]
    ProtocolVersion(u32),
    PublishChord(Chord),
    PublishPitches(HashSet<Note>),
    PublishSilence,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerToPublisher {
    Pong,
    NowAreYou,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsumerToServer {
    IAmConsumer {
        id: String,
    },
    #[serde(rename = "ConsumerProtocolVersion")]
    ProtocolVersion(u32),
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerToConsumer {
    ChordEvent(Chord),
    PitchesEvent(HashSet<Note>),
    Silence,
    Pong,
}

#[cfg(test)]
mod test {
    use klib::core::{chord::Chordable, named_pitch::NamedPitch, note, octave::Octave};

    use super::*;

    #[test]
    fn serializes_announce() {
        let message = ConsumerToServer::IAmConsumer {
            id: "Hello there".to_string(),
        };
        dbg!(serde_json::to_string_pretty(&message).unwrap());
    }

    #[test]
    fn serializes_piano_announce() {
        let message = PublisherToServer::IAmPublisher {
            id: "Hello there".to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&message).unwrap());
    }

    #[test]
    fn serializes_piano_chord() {
        let chord = Chord::new(note::AFlat).sus4().seven().add13();
        let message = PublisherToServer::PublishChord(chord);
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
        let message = PublisherToServer::PublishPitches(chord);
        println!("{:?}", serde_json::to_string(&message).unwrap());
    }
}
