use super::phase::Phase;
use super::{WaveGenerator, TAU};
use crate::notes::lerp;

pub struct Sawtooth {
    phase: Phase,
}

impl WaveGenerator for Sawtooth {
    fn new(sample_rate: f32) -> Sawtooth {
        Sawtooth {
            phase: Phase::new(sample_rate),
        }
    }

    fn before(&mut self, rel_midi_note: i8) {
        self.phase.before(rel_midi_note);
    }

    #[inline]
    fn next(&mut self, sample_idx: f32) -> f32 {
        let phase = self.phase.next(sample_idx);
        let pos = (phase % TAU) / (TAU * 0.5);
        if pos < 1.0 {
            lerp(-1.0, 0.0, pos - 0.0)
        } else {
            lerp(0.0, 1.0, pos - 1.0)
        }
    }

    fn after(&mut self, buf_len: f32) {
        self.phase.after(buf_len);
    }
}
