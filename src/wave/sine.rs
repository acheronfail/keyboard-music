use super::phase::Phase;
use super::WaveGenerator;

pub struct Sine {
    phase: Phase,
}

impl WaveGenerator for Sine {
    fn new(sample_rate: f32) -> Sine {
        Sine {
            phase: Phase::new(sample_rate),
        }
    }

    #[inline]
    fn before(&mut self, rel_midi_note: i8) {
        self.phase.before(rel_midi_note);
    }

    #[inline]
    fn next(&mut self, sample_idx: f32) -> f32 {
        self.phase.next(sample_idx).sin()
    }

    #[inline]
    fn after(&mut self, buf_len: f32) {
        self.phase.after(buf_len);
    }
}
