use std::time::Duration;

use either::Either;
use klib::core::{base::Playable, chord::Chord};
use tokio::sync::mpsc;

use crate::pitches::Pitches;

pub fn run(mut rx: mpsc::Receiver<Either<Chord, Pitches>>) -> anyhow::Result<()> {
    // .play(
    let mut handle = None;
    while let Some(msg) = rx.blocking_recv() {
        match msg {
            Either::Left(chord) => {
                handle = Some(chord.play(
                    Duration::ZERO,
                    Duration::from_secs(5),
                    Duration::from_millis(500),
                ))
            }
            Either::Right(pitches) => {
                handle = Some(pitches.play(
                    Duration::ZERO,
                    Duration::from_secs(5),
                    Duration::from_millis(500),
                ))
            }
        }
    }
    drop(handle);
    Ok(())
}
