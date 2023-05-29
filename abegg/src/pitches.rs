use std::collections::HashSet;

use klib::core::{
    base::{Playable, PlaybackHandle, Res as KlibResult},
    note::Note,
    pitch::HasFrequency,
};

#[derive(Debug, Clone, Default)]
pub struct Pitches(HashSet<Note>);

impl From<HashSet<Note>> for Pitches {
    fn from(value: HashSet<Note>) -> Self {
        Self(value)
    }
}

impl Playable for Pitches {
    fn play(&self, delay: f32, length: f32, fade_in: f32) -> KlibResult<PlaybackHandle> {
        use rodio::{source::SineWave, OutputStream, Sink, Source};
        use std::time::Duration;

        let chord_tones = &self.0;

        if length <= chord_tones.len() as f32 * delay {
            return Err(anyhow::Error::msg(
                "The delay is too long for the length of play (i.e., the number of chord tones times the delay is longer than the length).",
            ));
        }

        let (stream, stream_handle) = OutputStream::try_default()?;

        let mut sinks = vec![];

        for (k, n) in chord_tones.iter().enumerate() {
            let sink = Sink::try_new(&stream_handle)?;

            let d = k as f32 * delay;

            let source = SineWave::new(n.frequency())
                .take_duration(Duration::from_secs_f32(length - d))
                .buffered()
                .delay(Duration::from_secs_f32(d))
                .fade_in(Duration::from_secs_f32(fade_in))
                .amplify(0.20);

            sink.append(source);

            sinks.push(sink);
        }

        Ok(PlaybackHandle::new(stream, stream_handle, sinks))
    }
}
