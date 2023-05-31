#![doc = include_str!("../README.md")]

use std::collections::HashSet;

use klib::core::{chord::Chord, note::Note};
use serde::{Deserialize, Serialize};
use tokio_websockets::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublisherMessage {
    IAmPublisher { id: String },
    PublishChord(Chord),
    PublishPitches(HashSet<Note>),
    Silence,
}

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
    ChordEvent(Chord),
    PitchesEvent(HashSet<Note>),
    Silence,
}

impl ConsumerMessage {
    #[must_use]
    pub fn to_message(self) -> Message {
        let message = serde_json::to_string_pretty(&self).expect("Serialization failed");
        Message::text(message)
    }
}

#[cfg(test)]
mod test {
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
        dbg!(serde_json::to_string_pretty(&message).unwrap());
    }

    #[test]
    fn serializes_piano_chord() {
        let chord = [
            Note::new(
                klib::core::named_pitch::NamedPitch::A,
                klib::core::octave::Octave::Eleven,
            ),
            Note::new(
                klib::core::named_pitch::NamedPitch::ASharp,
                klib::core::octave::Octave::Five,
            ),
        ]
        .into_iter()
        .collect();
        let chord = None;
        let message = PublisherMessage::PublishChord(chord);
        dbg!(serde_json::to_string_pretty(&message).unwrap());
    }
}
