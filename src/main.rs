#![recursion_limit = "256"]

mod keymap;
mod notes;
mod stream;
#[cfg(feature = "visualiser")]
mod vis;
mod wave;

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait};
use device_query::{DeviceQuery, DeviceState};
use keymap::KeyMap;
use notes::Notes;
use stream::StreamWrapper;
use wave::Wave;

pub type MidiNote = u8;

#[derive(Debug, Copy, Clone)]
pub enum Action {
    NextWave,
}

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

    /// Choose which type of wave to play
    #[clap(short = 'w', long = "wave", value_enum, default_value_t = Wave::default())]
    pub wave: Wave,
}

fn main() -> Result<()> {
    let args = Args::parse();

    #[cfg(feature = "visualiser")]
    {
        use std::sync::mpsc;

        if args.show_visualiser {
            let (audio_tx, audio_rx) = mpsc::channel();
            let (option_tx, option_rx) = mpsc::channel();
            std::thread::spawn(move || {
                if let Err(e) = audio_loop(args, Some(audio_tx), Some(option_rx)) {
                    panic!("{}", e);
                }
            });

            vis::open_and_run(audio_rx, option_tx);
        }
    }

    audio_loop(args, None, None)?;
    Ok(())
}

fn audio_loop(
    args: Args,
    audio_tx: Option<Sender<Vec<f32>>>,
    option_rx: Option<Receiver<Action>>,
) -> Result<()> {
    #[cfg(not(feature = "visualiser"))]
    {
        drop(audio_tx);
        drop(option_rx);
    }

    // audio setup
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(anyhow!("No output device available"))?;
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate().0 as f32;

    // shared data (audio thread + keyboard thread) of which keycodes are currently active
    let active_keys = Arc::new(Mutex::new(Vec::<MidiNote>::new()));
    let notes = Arc::new(Mutex::new(Notes::new(
        &args.keymap,
        args.wave,
        sample_rate,
    )?));

    // create audio stream and start playing it immediately
    let mut stream = StreamWrapper::new(device.build_output_stream(
        &config.into(),
        {
            let notes = notes.clone();
            let active_keys = active_keys.clone();
            move |buf: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // this buffer isn't zeroed on all platforms
                buf.fill(0.0);

                // check shared mutex to update key state
                {
                    let mut notes = notes.lock().unwrap();

                    {
                        notes.update_keys(&*active_keys.lock().unwrap());
                    }

                    notes.generate_audio(buf);
                }

                // send a copy of the audio buffer over to the visualiser
                #[cfg(feature = "visualiser")]
                if let Some(ref tx) = audio_tx {
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

        #[cfg(feature = "visualiser")]
        if let Some(rx) = &option_rx {
            if let Ok(opt) = rx.try_recv() {
                match opt {
                    Action::NextWave => notes.lock().unwrap().update_wave(),
                }
            }
        }

        std::thread::sleep(KEYPRESS_INTERVAL);
    }
}
