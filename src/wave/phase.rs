use super::{WaveGenerator, FREQ_FACTOR, PITCH_FACTOR, TAU};

/// This just calculates and keeps track of phases for all any currently playing
/// notes. Doesn't produce any audio by itself, but it's used as a building
/// block for other waves.
pub struct Phase {
    current_note: i8,
    current_phase: f32,

    note_phases: [f32; u8::MAX as usize],

    base_factor: f32,
    wave_factor: f32,
}

impl Phase {
    #[inline]
    fn note_idx(&self, rel_midi_note: i8) -> usize {
        (rel_midi_note + i8::MAX) as usize
    }
}

impl WaveGenerator for Phase {
    fn new(sample_rate: f32) -> Phase {
        Phase {
            current_note: 0,
            current_phase: 0.0,

            note_phases: [0.0; u8::MAX as usize],

            base_factor: FREQ_FACTOR / sample_rate,
            wave_factor: 0.0,
        }
    }

    #[inline]
    fn clear(&mut self, rel_midi_note: i8) {
        self.note_phases[self.note_idx(rel_midi_note)] = 0.0;
    }

    #[inline]
    fn before(&mut self, rel_midi_note: i8) {
        self.wave_factor = self.base_factor * PITCH_FACTOR.powf(rel_midi_note as f32);
        self.current_phase = self.note_phases[self.note_idx(rel_midi_note)];
        self.current_note = rel_midi_note;
    }

    #[inline]
    fn next(&mut self, sample_idx: f32) -> f32 {
        self.current_phase + sample_idx * self.wave_factor
    }

    #[inline]
    fn after(&mut self, buf_len: f32) {
        self.note_phases[self.note_idx(self.current_note)] = self.next(buf_len) % TAU;
    }
}
