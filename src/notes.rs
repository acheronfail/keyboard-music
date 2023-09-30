use std::collections::HashMap;
use std::f32::consts::PI;

use anyhow::Result;

use crate::keymap::{self, KeyMap, KeyToNote, NoteToKey};

/// Max audio volume used when generating the audio data
pub const MAX_VOLUME: f32 = 0.5;
/// Middle "A" - all note frequency calculations are relative to this
pub const BASE_NOTE_FREQ: f32 = 440.0;
/// Midi pitch number of `BASE_NOTE_FREQ`, must be kept in sync with it
const MIDI_OFFSET: i8 = 69;

/// PI times 2 - what more do you want from this comment?
const TAU: f32 = 2.0 * PI;
/// Used to calculate the pitch for a given note
const FREQ_FACTOR: f32 = BASE_NOTE_FREQ * TAU;

pub struct Notes {
    notes: HashMap<i8, NoteState>,

    note_to_key: NoteToKey,
    key_to_note: KeyToNote,

    base_factor: f32,
    pitch_factor: f32,
}

impl Notes {
    pub fn new(keymap: &KeyMap, sample_rate: f32) -> Result<Notes> {
        let (key_to_note, note_to_key) = keymap::generate_maps(keymap)?;
        Ok(Notes {
            notes: HashMap::new(),
            note_to_key,
            key_to_note,
            base_factor: FREQ_FACTOR / sample_rate,
            pitch_factor: 2_f32.powf(1.0 / 12.0),
        })
    }

    pub fn update_keys(&mut self, active_keys: &[u8]) {
        // flag any notes that should no longer be playing as inactive
        self.notes.retain(|rel_midi_note, state| {
            // are any keys that map to this note still playing?
            let is_still_active = self.note_to_key[(*rel_midi_note + MIDI_OFFSET) as usize]
                .iter()
                .any(|keycode| active_keys.contains(keycode));

            state.active = is_still_active;
            state.active || state.volume > 0.0
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

    // TODO: abstract out waveform generation code so we can choose different ones (sine, saw, square, triangle, etc)
    pub fn generate_audio(&mut self, buf: &mut [f32]) {
        let note_volume_ratio = MAX_VOLUME / self.notes.len() as f32;
        for (rel_midi_note, note_state) in self.notes.iter_mut() {
            // generate sine pitch of this note
            let wave_factor = self.base_factor * self.pitch_factor.powf(*rel_midi_note as f32);

            // the volume this note should be fading towards
            let target_volume = note_state.get_target_volume(note_volume_ratio);

            // update audio buffer with this note's wave
            let buf_len = buf.len() as f32;
            for (idx, sample) in buf.iter_mut().enumerate() {
                let wave = (note_state.phase + idx as f32 * wave_factor).sin();
                let t = idx as f32 / buf_len;
                *sample += wave * note_state.get_volume(t, target_volume)
            }

            // set the note to its target volume
            note_state.set_volume(target_volume);

            // update this note's phase
            note_state.phase = (note_state.phase + buf.len() as f32 * wave_factor) % TAU;
        }
    }
}

struct NoteState {
    active: bool,
    phase: f32,
    volume: f32,
}

impl NoteState {
    fn new() -> Self {
        Self {
            active: true,
            phase: 0.0,
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
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    (1.0 - t) * a + t * b
}
