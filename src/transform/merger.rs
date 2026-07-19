use rustfft::num_complex::Complex;

pub trait Merger {
    fn merge(&self, frequency_bins: &[Complex<f32>]) -> Vec<f32>;
}

///linear mode where each output 'bar' is created via averaging a fixed number of fft 'frequency bins'
pub struct LinearMerger {
    ///number of fft output frequency bins to merge into each bar in the output graph
    bins_per_bar: usize,
    ///index into frequency bins which are useful (as real input FFT mirros halfway so we only want first half)
    useful_bins: usize,
}

impl Merger for LinearMerger {
    fn merge(&self, frequency_bins: &[Complex<f32>]) -> Vec<f32> {
        // copy the real part of the transform into the output
        // could potentially add other bar merging options like max or a weighted average
        // no point dividing for average since bars are normalised anyway
        frequency_bins[..self.useful_bins]
            .chunks_exact(self.bins_per_bar)
            .map(|bins| bins.iter().map(|bin| bin.norm_sqr()).sum::<f32>())
            .collect()
    }
}

impl LinearMerger {
    pub fn new(input_bins: usize, output_bars: usize) -> LinearMerger {
        // only half the output frequency bins are used since fourier transform of real only input is mirrored
        let useful_bins = input_bins / 2;
        let bins_per_bar = useful_bins / output_bars;

        Self {
            bins_per_bar,
            useful_bins,
        }
    }
}

///exponential mode where the number of fft 'frequency bins' per output 'bar' increases exponentially
///this replicates the way the human ear hears sounds and thus traditional musical notes as the frequency of sound doubles with each octave

pub struct ExponentialMerger {
    ///number of fft output frequency bins to merge for each bar in the output graph
    bins_per_bar: Vec<usize>,

    /// the number of output bars (equal to bins_per_bar.len())
    output_bars: usize,

    /// first frequency bin to use
    start_bin: usize,
    /// last frequency bin to use
    ///index into frequency bins which are useful (as real input FFT mirrors halfway so we only want first half)
    end_bin: usize,
}

impl Merger for ExponentialMerger {
    fn merge(&self, frequency_bins: &[Complex<f32>]) -> Vec<f32> {
        let mut bars: Vec<f32> = Vec::with_capacity(self.output_bars);

        let mut start: usize = self.start_bin;
        let mut end: usize = start;

        for bar_width in &self.bins_per_bar {
            end += bar_width;
            bars.push(
                frequency_bins[start..end]
                    .iter()
                    .map(|c| c.norm_sqr())
                    .sum::<f32>()
                    / bar_width.clone() as f32,
            );
            start = end;
        }

        bars
    }
}

impl ExponentialMerger {
    const DEFAULT_OCTAVE_RANGE: u32 = 8;
    const DEFAULT_TUNING_FREQUENCY: f64 = 440_f64;
    const DEFAULT_STARTING_NOTE_OFFSET: isize = -48;

    ///create an exponential merger with default settings.
    ///it is unlikely that the default settings will produce perfect tone or semitone bars
    pub fn new_auto(input_bins: usize, output_bars: usize, sample_rate: u32) -> ExponentialMerger {
        Self::new_custom_function(
            input_bins,
            output_bars,
            sample_rate,
            ExponentialMerger::DEFAULT_TUNING_FREQUENCY,
            output_bars / ExponentialMerger::DEFAULT_OCTAVE_RANGE as usize,
            ExponentialMerger::DEFAULT_STARTING_NOTE_OFFSET,
        )
    }

    ///create a new exponential merger defining a custom function for distributing frequency bins to bars
    ///the highest frequency will be equal to the lowest frequency * 2^(number of output bars / bars per octave)
    /// the lowest frequency will be the frequency of the note offset below the tuning frequency
    pub fn new_custom_function(
        input_bins: usize,
        output_bars: usize,
        sample_rate: u32,
        tuning_frequency: f64,
        bars_per_octave: usize,
        starting_note_offset: isize,
    ) -> ExponentialMerger {
        let builder = ExponentialMergerBuildHelper {
            input_bins,
            output_bars,
            sample_rate,
            tuning_frequency,
            bars_per_octave,
            starting_note_offset,
            starting_bin_offset: starting_note_offset as f64 - 0.5,
        };

        let mut bins_per_bar: Vec<usize> = Vec::with_capacity(output_bars);

        let useful_bins = input_bins / 2;
        let start = builder.frequency_to_bin(builder.note_to_frequency(-1_f64));

        let mut start_bin = start;

        for bar in 0..output_bars {
            let end_bin = builder.frequency_to_bin(builder.note_to_frequency(bar as f64));
            bins_per_bar.push(end_bin - start_bin);
            start_bin = end_bin;
        }

        ExponentialMerger {
            bins_per_bar,
            output_bars,

            start_bin: start,
            end_bin: useful_bins,
        }
    }
}

///"notes" and "bars" are sort of used interchageably here since the assumption is that the output bars represent notes
struct ExponentialMergerBuildHelper {
    input_bins: usize,
    output_bars: usize,

    ///pipewire sample rate
    sample_rate: u32,

    /// the number of notes in an octave (i.e. the number of notes for each doubling of frequency)
    /// set this to 12 to display semitones or 6 to average to full tones etc
    bars_per_octave: usize,

    ///frequency of a note that all other notes will set relative to
    tuning_frequency: f64,

    ///how many bars below the tuning frequency your first note is
    ///this only really exists so that the tuning frequency can be set independently of the first note to display- e.g. tune A4 at 440hz but the "0th" note is A0 (offset -48) at 27.5hz
    ///there is no difference in practice between doing this and setting the tuning frequency at 27.5hz with zero offset
    starting_note_offset: isize,

    ///this is just the starting note offset -0.5 since we want a bar to represent frequency bins from halfway between the previous note to halfway until the next note.
    starting_bin_offset: f64,
}

impl ExponentialMergerBuildHelper {
    fn note_to_frequency(&self, frequency: f64) -> f64 {
        self.tuning_frequency
            * 2_f64
                .powf((frequency + self.starting_bin_offset as f64) / self.bars_per_octave as f64)
    }

    fn frequency_to_note(&self, note: f64) -> f64 {
        (self.bars_per_octave as f64 * f64::ln(note / self.tuning_frequency as f64) / 2.0_f64.ln())
            - self.starting_bin_offset as f64
    }

    fn bin_to_frequency(&self, bin: usize) -> f64 {
        ((self.sample_rate as usize / self.input_bins) * bin) as f64
    }

    fn frequency_to_bin(&self, frequency: f64) -> usize {
        ((self.input_bins as f64 / self.sample_rate as f64) * frequency as f64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_merger_configuration_and_merge() {
        let merger = LinearMerger::new(12, 3);

        assert_eq!(merger.bins_per_bar, 2);
        assert_eq!(merger.useful_bins, 6);

        let frequency_bins = vec![
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 2.0),
            Complex::new(3.0, 4.0),
            Complex::new(-1.0, 2.0),
            Complex::new(2.0, -2.0),
            Complex::new(0.5, 0.5),
            Complex::new(10.0, 10.0),
            Complex::new(10.0, 10.0),
            Complex::new(10.0, 10.0),
            Complex::new(10.0, 10.0),
            Complex::new(10.0, 10.0),
            Complex::new(10.0, 10.0),
        ];

        assert_eq!(merger.merge(&frequency_bins), vec![5.0, 30.0, 8.5]);
    }

    #[test]
    fn exponential_merger_configuration_and_merge() {
        let merger = ExponentialMerger::new(vec![1, 2, 3], 6, 3);

        let frequency_bins = vec![
            Complex::new(3.0, 4.0),
            Complex::new(1.0, 2.0),
            Complex::new(2.0, 0.0),
            Complex::new(0.5, 0.5),
            Complex::new(0.0, 3.0),
            Complex::new(-2.0, -1.0),
        ];

        assert_eq!(merger.merge(&frequency_bins), vec![25.0, 9.0, 14.5]);
    }

    #[test]
    fn exponential_merger_builder_configuration_merge() {
        let merger = ExponentialMerger::new_custom_function(64, 4, 64, 0.25, 1, 0);

        assert_eq!(merger.bins_per_bar, vec![0, 2, 3, 4]);
        assert_eq!(merger.useful_bins, 32);
        assert_eq!(merger.output_bars, 4);

        let frequency_bins = vec![
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 2.0),
            Complex::new(3.0, 0.0),
            Complex::new(0.0, 4.0),
            Complex::new(1.0, 2.0),
            Complex::new(2.0, 2.0),
            Complex::new(0.0, 3.0),
            Complex::new(-1.0, 0.0),
            Complex::new(1.0, 1.0),
        ];

        assert_eq!(merger.merge(&frequency_bins), vec![0.0, 5.0, 30.0, 20.0]);
    }
}
