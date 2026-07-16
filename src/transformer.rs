use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::{process::Output, sync::Arc};

pub struct Transformer {
    input_samples: usize,
    output_bars: usize,
    bins_per_bar: usize,
    fft: Arc<dyn Fft<f32>>,
    input_buffer: Vec<Complex<f32>>,
}

impl Transformer {
    pub fn new(input_samples: usize, output_bars: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(input_samples);
        let mut input_buffer = vec![Complex::new(0.0, 0.0); input_samples];
        let bins_per_bar = (input_samples / 2) / output_bars;

        Self {
            input_samples,
            output_bars,
            bins_per_bar,
            fft,
            input_buffer,
        }
    }

    pub fn transform(&mut self, input: &[f32]) -> Vec<f32> {
        // copy the input into the real part of complex numbers in the input buffer
        for i in 0..self.input_samples {
            self.input_buffer[i] = Complex::new(input[i], 0.0)
        }

        self.fft.process(&mut self.input_buffer);

        // copy the real part of the transform into the output
        // only half is copied since fourier transform of real only input is a mirrored
        self.input_buffer[..self.input_samples / 2]
            .chunks(self.bins_per_bar)
            .map(|bins| bins.iter().map(|bin| bin.norm_sqr()).sum::<f32>() / bins.len() as f32)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_frequency_7_hz() {
        let input = vec![
            0.0_f32, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0,
            0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0, 0.0,
        ];

        let mut t = Transformer::new(input.len(), input.len() / 2);

        let result = t.transform(&input);

        print!("{:?}", result);

        let max_index = result
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
            .unwrap();

        assert_eq!(7_usize, max_index)
    }
}
