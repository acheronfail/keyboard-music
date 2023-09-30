#![recursion_limit = "256"]

mod keymap;
mod notes;
mod stream;
#[cfg(feature = "visualiser")]
mod vis;

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait};
use device_query::{DeviceQuery, DeviceState};
use keymap::KeyMap;
use notes::Notes;
use stream::StreamWrapper;

pub type MidiNote = u8;

/// If no keys are pressed for this amount of time, the stream will stop playing
/// to reduce resource usage (it will resume when keys are pressed again)
const INACTIVITY_TIME: Duration = Duration::from_secs(5);
/// How often to check for keyboard activity
const KEYPRESS_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, Parser)]
pub struct Args {
    /// Which keymap to use
    #[clap(short = 'k', long = "keymap", value_enum, default_value_t = KeyMap::default())]
    pub keymap: KeyMap,

    /// Whether to show a wave visualiser
    #[cfg(feature = "visualiser")]
    #[clap(short = 'v', long = "vis")]
    pub show_visualiser: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    #[cfg(feature = "visualiser")]
    {
        if args.show_visualiser {
            let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();
            std::thread::spawn(move || {
                if let Err(e) = audio_loop(args, Some(tx)) {
                    panic!("{}", e);
                }
            });

            vis::open_and_run(rx);
        }
    }

    audio_loop(args, None)?;
    Ok(())
}

fn audio_loop(args: Args, tx: Option<Sender<Vec<f32>>>) -> Result<()> {
    #[cfg(not(feature = "visualiser"))]
    drop(tx);

    // audio setup
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(anyhow!("No output device available"))?;
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate().0 as f32;

    // shared data (audio thread + keyboard thread) of which keycodes are currently active
    let active_keys = Arc::new(Mutex::new(Vec::<MidiNote>::new()));
    // create audio stream and start playing it immediately
    let mut stream = StreamWrapper::new(device.build_output_stream(
        &config.into(),
        {
            // this is a map of (relative midi note -> (note phase position, note volume))
            let mut notes = Notes::new(&args.keymap, sample_rate)?;
            let active_keys = active_keys.clone();
            move |buf: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // this buffer isn't zeroed on all platforms
                buf.fill(0.0);

                // check shared mutex to update key state
                {
                    notes.update_keys(&*active_keys.lock().unwrap());
                }

                notes.generate_audio(buf);

                // send a copy of the audio buffer over to the visualiser
                #[cfg(feature = "visualiser")]
                if let Some(ref tx) = tx {
                    let _ = tx.send(buf.to_vec());
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
