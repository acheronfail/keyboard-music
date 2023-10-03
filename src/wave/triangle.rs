use super::phase::Phase;
use super::{WaveGenerator, TAU};
use crate::notes::lerp;

pub struct Triangle {
    phase: Phase,
}

impl WaveGenerator for Triangle {
    fn new(sample_rate: f32) -> Triangle {
        Triangle {
            phase: Phase::new(sample_rate),
        }
    }

    fn before(&mut self, rel_midi_note: i8) {
        self.phase.before(rel_midi_note);
    }

    #[inline]
    fn next(&mut self, sample_idx: f32) -> f32 {
        let phase = self.phase.next(sample_idx);
        let pos = (phase % TAU) / (TAU * 0.25);
        if pos < 1.0 {
            lerp(0.0, 1.0, pos - 0.0)
        } else if pos < 2.0 {
            lerp(1.0, 0.0, pos - 1.0)
        } else if pos < 3.0 {
            lerp(0.0, -1.0, pos - 2.0)
        } else {
            lerp(-1.0, 0.0, pos - 3.0)
        }
    }

    fn after(&mut self, buf_len: f32) {
        self.phase.after(buf_len);
    }
}
