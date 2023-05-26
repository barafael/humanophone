use std::collections::HashSet;

use klib::core::{chord::Chord, note::Note};
use serde::{Deserialize, Serialize};
use tokio_websockets::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublisherMessage {
    IAmPublisher { id: String },
    PublishChord(HashSet<Note>, Option<Chord>),
}

impl PublisherMessage {
    pub fn to_message(self) -> Message {
        let message = serde_json::to_string_pretty(&self).unwrap();
        Message::text(message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsumerMessage {
    IAmConsumer { id: String },
    ChordEvent(HashSet<Note>, Option<Chord>),
}

impl ConsumerMessage {
    pub fn to_message(self) -> Message {
        let message = serde_json::to_string_pretty(&self).unwrap();
        Message::text(message)
    }
}
