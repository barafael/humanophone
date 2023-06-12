use std::{collections::HashSet, time::Duration};

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
    fn play(
        &self,
        delay: Duration,
        length: Duration,
        fade_in: Duration,
    ) -> KlibResult<PlaybackHandle> {
        use rodio::{source::SineWave, OutputStream, Sink, Source};

        let chord_tones = &self.0;

        if length <= delay * chord_tones.len() as u32 {
            return Err(anyhow::Error::msg(
                "The delay is too long for the length of play (i.e., the number of chord tones times the delay is longer than the length).",
            ));
        }

        let (stream, stream_handle) = OutputStream::try_default()?;

        let mut sinks = vec![];

        for (k, n) in chord_tones.iter().enumerate() {
            let sink = Sink::try_new(&stream_handle)?;

            let d = delay * k as u32;

            let source = SineWave::new(n.frequency())
                .take_duration(length - d)
                .buffered()
                .delay(d)
                .fade_in(fade_in)
                .amplify(0.20);

            sink.append(source);

            sinks.push(sink);
        }

        Ok(PlaybackHandle::new(stream, stream_handle, sinks))
    }
}
