use serde::Serialize;
use tokio_websockets::Message;

pub trait ToMessage {
    #[must_use]
    fn to_message(self) -> Message;
}

#[cfg(feature = "message")]
impl<T> ToMessage for T
where
    T: Serialize,
{
    #[must_use]
    fn to_message(self) -> Message {
        let message = serde_json::to_string_pretty(&self).expect("Serialization failed");
        Message::text(message)
    }
}
