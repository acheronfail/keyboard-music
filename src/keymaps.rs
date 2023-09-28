use std::collections::HashMap;
use std::str::FromStr;

use clap::ValueEnum;
use device_query::Keycode;
use serde_json::{json, Value};

use crate::{Args, MidiNote, Result};

pub type KeyToNote = Vec<Option<MidiNote>>;
pub type NoteToKey = Vec<Vec<MidiNote>>;

pub fn generate_maps(args: &Args) -> Result<(KeyToNote, NoteToKey)> {
    let keymap: HashMap<String, Option<MidiNote>> = serde_json::from_value(args.keymap.get_map())?;

    let mut key_to_note = vec![None; MidiNote::MAX as usize];
    for (keycode_str, note) in keymap {
        let keycode = Keycode::from_str(&keycode_str)?;
        key_to_note[keycode as usize] = note;
    }

    let mut note_to_key = vec![vec![]; MidiNote::MAX as usize];
    for (keycode, note) in key_to_note.iter().enumerate() {
        if let Some(note) = note {
            note_to_key[*note as usize].push(keycode as MidiNote);
        }
    }

    Ok((key_to_note, note_to_key))
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum KeyMaps {
    Us,
    Piano,
}

impl KeyMaps {
    fn get_map(&self) -> Value {
        match self {
            KeyMaps::Us => json!({
              "Escape": 45,
              "F1": 46,
              "F2": 47,
              "F3": 48,
              "F4": 49,
              "F5": 50,
              "F6": 51,
              "F7": 52,
              "F8": 53,
              "F9": 54,
              "F10": 55,
              "F11": 56,
              "F12": 57,
              "Insert": 58,
              "Delete": 59,
              "Grave": 60,
              "Key1": 61,
              "Key2": 62,
              "Key3": 63,
              "Key4": 64,
              "Key5": 65,
              "Key6": 66,
              "Key7": 67,
              "Key8": 68,
              "Key9": 69,
              "Key0": 70,
              "Minus": 71,
              "Equal": 72,
              "Backspace": 73,
              "Tab": 74,
              "Q": 75,
              "W": 76,
              "E": 77,
              "R": 78,
              "T": 79,
              "Y": 80,
              "U": 81,
              "I": 82,
              "O": 83,
              "P": 84,
              "LeftBracket": 85,
              "RightBracket": 86,
              "BackSlash": 87,
              "CapsLock": 88,
              "A": 89,
              "S": 90,
              "D": 91,
              "F": 92,
              "G": 93,
              "H": 94,
              "J": 95,
              "K": 96,
              "L": 97,
              "Semicolon": 98,
              "Apostrophe": 99,
              "Enter": 100,
              "LShift": 101,
              "Z": 102,
              "X": 103,
              "C": 104,
              "V": 105,
              "B": 106,
              "N": 107,
              "M": 108,
              "Comma": 109,
              "Dot": 110,
              "Slash": 111,
              "RShift": 112,
              "LControl": 113,
              "Meta": 114,
              "LAlt": 115,
              "Space": 116,
              "RAlt": 117,
              "RControl": 118,
              "Left": 119,
              "Up": 120,
              "Down": 121,
              "Right": 122
            }),
            KeyMaps::Piano => json!({
              "Q": 48,
              "Key2": 49,
              "W": 50,
              "Key3": 51,
              "E": 52,
              "R": 53,
              "Key5": 54,
              "T": 55,
              "Key6": 56,
              "Y": 57,
              "Key7": 58,
              "U": 59,
              "I": 60,
              "Key9": 61,
              "O": 62,
              "Key0": 63,
              "P": 64,
              "Z": 60,
              "S": 61,
              "X": 62,
              "D": 63,
              "C": 64,
              "V": 65,
              "G": 66,
              "B": 67,
              "H": 68,
              "N": 69,
              "J": 70,
              "M": 71,
              "Comma": 72,
              "L": 73,
              "Dot": 74,
              "Semicolon": 75,
              "Slash": 76
            }),
        }
    }
}
