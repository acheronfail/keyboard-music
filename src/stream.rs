use anyhow::Result;
use cpal::traits::StreamTrait;
use cpal::Stream;

pub struct StreamWrapper {
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
