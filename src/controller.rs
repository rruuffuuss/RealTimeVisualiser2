use super::{captor, display::Display, normaliser::Normaliser, transformer::Transformer};

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use std::collections::VecDeque;
use std::thread::{self, Thread};
use std::usize;

pub enum Mode {
    ///single large FFT with enough samples to cover the full frequency range
    Monolithic,
    ///samples are recursively decimated (low pass filter then downsample) by factor 2 for lower frequency resolution
    Decimating,
}

pub struct Controller {
    display_height: u16,
    display_width: u16,
    /// display_width and target_framerate control how 'complete' the frequency spectrum is
    /// a window width of sample rate / framerate will result in all samples being processed.
    /// a short window width and a low framerate will mean the frequency spectrum is based on a snapshot of the audio playing, and some parts may be missed
    /// a long window width and a high framerate will mean the frequency spectrum calculated for a frame uses some samples included in previous frames
    ///number of samples to analyse, must be power of 2
    sample_window: usize,
    ///frequency of analysis & display update events in Hz
    ///4does nothing currently
    target_framerate: u16,
    /// number of output 'bins' for the output frequency spectrum
    /// just using equidistant bins for now, will update for musical notes later
    /// down the line this may end up as functions defining the window size and frequency for each output bin individually
    /// would need to buffer for the largest
    bars: usize,

    min_freq: usize,
    max_freq: usize,

    ///number of output graphs
    ///display_grid: (u8, u8),
    ///channel map for input channels to output graphs
    ///input channels within an inner vector are averaged together
    mode: Mode,
    // channel_map: Vec<(Vec<u8>, u8)>,
}

impl Controller {
    pub fn new(
        display_height: u16,
        display_width: u16,
        sample_window: usize,
        target_framerate: u16,
        min_freq: usize,
        max_freq: usize,
        mode: Mode,
    ) -> Self {
        Self {
            display_height,
            display_width,
            sample_window,
            target_framerate,
            bars: display_width as usize,
            min_freq,
            max_freq,
            mode,
        }
    }

    pub fn run(&self) {
        let (fresh_tx, fresh_rx): (Sender<VecDeque<f32>>, Receiver<VecDeque<f32>>) =
            mpsc::channel();
        let (stale_tx, stale_rx): (Sender<VecDeque<f32>>, Receiver<VecDeque<f32>>) =
            mpsc::channel();

        let mut transformer =
            Transformer::new(self.sample_window as usize, self.display_width as usize);
        let mut normaliser = Normaliser::new(1.0_f32);
        let mut display = Display::new(self.display_width as u16, self.display_height as u16);

        let transform_buffer = VecDeque::from(vec![0.0_f32; self.sample_window]);

        stale_tx.send(transform_buffer).unwrap();

        thread::spawn(move || captor::run(fresh_tx, stale_rx));

        for recieved in fresh_rx {
            let mut spectrum_data = transformer.transform_split(recieved.as_slices());
            normaliser.normalise(&mut spectrum_data);
            display.display(&spectrum_data[..self.bars]);

            stale_tx.send(recieved).unwrap();
        }
    }
}
