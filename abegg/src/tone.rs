use klib::core::{
    base::{Playable, PlaybackHandle},
    note::Note,
    pitch::HasFrequency,
};

#[derive(Debug, Clone)]
pub struct Tone(Note);

impl From<Note> for Tone {
    fn from(value: Note) -> Self {
        Self(value)
    }
}

impl Playable for Tone {
    fn play(
        &self,
        delay: f32,
        length: f32,
        fade_in: f32,
    ) -> klib::core::base::Res<klib::core::base::PlaybackHandle> {
        use rodio::{source::SineWave, OutputStream, Sink, Source};
        use std::time::Duration;

        let (stream, stream_handle) = OutputStream::try_default()?;

        let sink = Sink::try_new(&stream_handle)?;

        let source = SineWave::new(self.0.frequency())
            .take_duration(Duration::from_secs_f32(length - delay))
            .buffered()
            .delay(Duration::from_secs_f32(delay))
            .fade_in(Duration::from_secs_f32(fade_in))
            .amplify(0.20);

        sink.append(source);

        Ok(PlaybackHandle::new(stream, stream_handle, vec![sink]))
    }
}
