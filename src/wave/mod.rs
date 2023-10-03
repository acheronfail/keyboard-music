mod phase;
mod sawtooth;
mod sine;
mod square;
mod triangle;

use std::f32::consts::PI;

use clap::ValueEnum;

use crate::notes::BASE_NOTE_FREQ;

/// PI times 2 - what more do you want from this comment?
const TAU: f32 = 2.0 * PI;
/// Used to calculate the pitch for a given note
const FREQ_FACTOR: f32 = BASE_NOTE_FREQ * TAU;
/// Used to calculate semi-tones between notes
/// This is the value of `2_f32.powf(1.0 / 12.0)`.
const PITCH_FACTOR: f32 = 1.0594630943592953;

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
pub enum Wave {
    #[default]
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

impl Wave {
    pub fn generator(&self, sample_rate: f32) -> Box<dyn WaveGenerator> {
        match self {
            Wave::Sine => Box::new(sine::Sine::new(sample_rate)),
            Wave::Square => Box::new(square::Square::new(sample_rate)),
            Wave::Triangle => Box::new(triangle::Triangle::new(sample_rate)),
            Wave::Sawtooth => Box::new(sawtooth::Sawtooth::new(sample_rate)),
        }
    }
}

pub trait WaveGenerator: Send {
    fn new(sample_rate: f32) -> Self
    where
        Self: Sized;

    fn clear(&mut self, _rel_midi_note: i8) {}

    fn before(&mut self, rel_midi_note: i8);
    fn next(&mut self, sample_idx: f32) -> f32;
    fn after(&mut self, buf_len: f32);
}
