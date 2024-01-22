use std::collections::HashMap;

use anyhow::Result;

use crate::keymap::{self, KeyMap, KeyToNote, NoteToKey};
use crate::wave::{Wave, WaveGenerator};

/// Max audio volume used when generating the audio data
pub const MAX_VOLUME: f32 = 0.5;
/// Middle "A" - all note frequency calculations are relative to this
pub const BASE_NOTE_FREQ: f32 = 440.0;
/// Midi pitch number of `BASE_NOTE_FREQ`, must be kept in sync with it
const MIDI_OFFSET: i8 = 69;

pub struct Notes {
    sample_rate: f32,
    notes: HashMap<i8, NoteState>,

    note_to_key: NoteToKey,
    key_to_note: KeyToNote,

    wave: Wave,
    wave_generator: Box<dyn WaveGenerator>,
}

impl Notes {
    pub fn new(keymap: &KeyMap, wave: Wave, sample_rate: f32) -> Result<Notes> {
        let (key_to_note, note_to_key) = keymap::generate_maps(keymap)?;
        Ok(Notes {
            sample_rate,
            notes: HashMap::new(),
            note_to_key,
            key_to_note,
            wave,
            wave_generator: wave.generator(sample_rate),
        })
    }

    #[allow(unused)]
    pub fn update_wave(&mut self) {
        self.wave = self.wave.next();
        self.wave_generator = self.wave.generator(self.sample_rate);
    }

    pub fn update_keys(&mut self, active_keys: &[u8]) {
        // flag any notes that should no longer be playing as inactive
        self.notes.retain(|rel_midi_note, state| {
            // are any keys that map to this note still playing?
            let is_still_active = self.note_to_key[(*rel_midi_note + MIDI_OFFSET) as usize]
                .iter()
                .any(|keycode| active_keys.contains(keycode));

            state.active = is_still_active;

            // tell the wave to drop its state for any active notes it's keeping track of
            let should_keep = state.active || state.volume > 0.0;
            if !should_keep {
                self.wave_generator.clear(*rel_midi_note);
            }

            should_keep
        });

        // start playing any new notes, or update existing ones
        for k in active_keys.iter() {
            if let Some(note) = self.key_to_note[*k as usize] {
                self.notes
                    .entry(note as i8 - MIDI_OFFSET)
                    .and_modify(|state| state.active = true)
                    .or_insert(NoteState::new());
            }
        }
    }

    pub fn generate_audio(&mut self, buf: &mut [f32]) {
        let note_volume_ratio = MAX_VOLUME / self.notes.len() as f32;
        for (rel_midi_note, note_state) in self.notes.iter_mut() {
            // the volume this note should be fading towards
            let target_volume = note_state.get_target_volume(note_volume_ratio);

            // update audio buffer with this note's wave
            let buf_len = buf.len() as f32;
            self.wave_generator.before(*rel_midi_note);
            for (idx, sample) in buf.iter_mut().enumerate() {
                let idx = idx as f32;
                let wave = self.wave_generator.next(idx);
                *sample += wave * note_state.get_volume(idx / buf_len, target_volume)
            }
            self.wave_generator.after(buf_len);

            // set the note to its target volume
            note_state.set_volume(target_volume);
        }
    }
}

struct NoteState {
    active: bool,
    volume: f32,
}

impl NoteState {
    fn new() -> Self {
        Self {
            active: true,
            volume: 0.0,
        }
    }

    fn set_volume(&mut self, target_volume: f32) {
        self.volume = target_volume;
    }

    fn get_volume(&self, t: f32, target_volume: f32) -> f32 {
        lerp(self.volume, target_volume, t * 3.0)
    }

    fn get_target_volume(&self, volume_ratio: f32) -> f32 {
        if self.active {
            volume_ratio
        } else {
            0.0
        }
    }
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    (1.0 - t) * a + t * b
}
