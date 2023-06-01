#![doc = include_str!("../README.md")]

use std::collections::HashSet;

use klib::core::{chord::Chord, note::Note};
use serde::{Deserialize, Serialize};
#[cfg(feature = "message")]
use tokio_websockets::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublisherMessage {
    IAmPublisher { id: String },
    PublishChord(Chord),
    PublishPitches(HashSet<Note>),
    Silence,
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
    ChordEvent(Chord),
    PitchesEvent(HashSet<Note>),
    Silence,
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
    use klib::core::{chord::Chordable, note};

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
}
