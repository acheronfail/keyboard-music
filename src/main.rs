#![recursion_limit = "256"]

mod keymaps;

use std::collections::HashMap;
use std::error::Error;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use device_query::{DeviceQuery, DeviceState};
use keymaps::KeyMaps;

pub type MidiNote = u8;
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Max audio volume used when generating the audio data
const MAX_VOLUME: f32 = 0.5;
/// Middle "A" - all note frequency calculations are relative to this
const BASE_NOTE_FREQ: f32 = 440.0;
/// Midi pitch number of `BASE_NOTE_FREQ`, must be kept in sync with it
const MIDI_OFFSET: i8 = 69;

/// If no keys are pressed for this amount of time, the stream will stop playing
/// to reduce resource usage (it will resume when keys are pressed again)
const INACTIVITY_TIME: Duration = Duration::from_secs(5);
/// How often to check for keyboard activity
const KEYPRESS_INTERVAL: Duration = Duration::from_millis(10);

/// PI times 2 - what more do you want from this comment?
const TAU: f32 = 2.0 * PI;
/// Used to calculate the pitch for a given note
const FREQ_FACTOR: f32 = BASE_NOTE_FREQ * TAU;

#[derive(Debug, Parser)]
pub struct Args {
    /// Which keymap to use
    #[clap(short = 'k', long = "keymap", value_enum, default_value_t = KeyMaps::default())]
    pub keymap: KeyMaps,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // this is a map of (relative midi note -> note phase position)
    let mut note_phases = HashMap::<i8, f32>::new();
    // shared data of which keycodes are currently active
    let active_keys = Arc::new(Mutex::new(Vec::<MidiNote>::new()));

    // read user keymap
    let (key_to_note, note_to_key) = keymaps::generate_maps(&args)?;

    // audio setup
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device available")?;
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate().0 as f32;

    // create audio stream and start playing it immediately
    // TODO: audio clips when notes stop, need to cleanly stop playing each note
    let mut stream = StreamWrapper::new(device.build_output_stream(
        &config.into(),
        {
            let base_factor = FREQ_FACTOR / sample_rate;
            let pitch_factor = 2_f32.powf(1.0 / 12.0);
            let active_keys = active_keys.clone();
            move |buf: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // this buffer isn't zeroed on all platforms
                buf.fill(0.0);

                // check which keys are pressed
                {
                    let active_keys = active_keys.lock().unwrap();

                    // remove any notes which should no longer be playing
                    note_phases.retain(|note, _| {
                        note_to_key[(*note + MIDI_OFFSET) as usize]
                            .iter()
                            .any(|keycode| active_keys.contains(keycode))
                    });

                    // start playing any new notes
                    for k in active_keys.iter() {
                        if let Some(note) = key_to_note[*k as usize] {
                            note_phases.entry(note as i8 - MIDI_OFFSET).or_insert(0.0);
                        }
                    }
                }

                // generate combined audio data for all notes
                let tone_volume = MAX_VOLUME / note_phases.len() as f32;
                for (relative_note, phase) in note_phases.iter_mut() {
                    // generate sin pitch of this note
                    let factor = base_factor * pitch_factor.powf(*relative_note as f32);
                    for (idx, sample) in buf.iter_mut().enumerate() {
                        *sample += (*phase + idx as f32 * factor).sin() * tone_volume;
                    }

                    // update this note's phase
                    *phase = (*phase + buf.len() as f32 * factor) % TAU;
                }
            }
        },
        |err| eprintln!("an error occurred on stream: {}", err),
        None,
    )?)?;

    // listen for keys
    let device_state = DeviceState::new();
    let mut last_key_press = Instant::now();
    loop {
        // query key state
        let keys = device_state.get_keys();

        // check if we should pause the stream due to inactivity
        if keys.len() > 0 {
            last_key_press = Instant::now();
            if stream.is_paused() {
                stream.play()?;
            }
        } else if last_key_press.elapsed() > INACTIVITY_TIME {
            stream.pause()?;
        }

        // acquire lock and update active keys so the audio thread can respond to it
        {
            let mut active_keys = active_keys.lock().unwrap();
            active_keys.drain(..);
            active_keys.extend(keys.iter().map(|k| *k as MidiNote));
        }

        std::thread::sleep(KEYPRESS_INTERVAL);
    }
}

struct StreamWrapper {
    stream: Stream,
    is_paused: bool,
}

impl StreamWrapper {
    pub fn new(stream: Stream) -> Result<StreamWrapper> {
        stream.play()?;
        Ok(StreamWrapper {
            stream,
            is_paused: false,
        })
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn play(&mut self) -> Result<()> {
        self.stream.play()?;
        self.is_paused = false;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<()> {
        self.stream.pause()?;
        self.is_paused = true;
        Ok(())
    }
}
