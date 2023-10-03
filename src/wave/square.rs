use super::sine::Sine;
use super::WaveGenerator;

pub struct Square {
    sine: Sine,
}

impl Square {
    const AMP: f32 = 0.5;
}

impl WaveGenerator for Square {
    fn new(sample_rate: f32) -> Square {
        Square {
            sine: Sine::new(sample_rate),
        }
    }

    fn before(&mut self, rel_midi_note: i8) {
        self.sine.before(rel_midi_note);
    }

    #[inline]
    fn next(&mut self, sample_idx: f32) -> f32 {
        let sine_wave = self.sine.next(sample_idx);
        if sine_wave < 0.0 {
            -Self::AMP
        } else if sine_wave > 0.0 {
            Self::AMP
        } else {
            0.0
        }
    }

    fn after(&mut self, buf_len: f32) {
        self.sine.after(buf_len);
    }
}
