pub struct Normaliser {
    max_threshold: f32,
}

impl Normaliser {
    pub fn new(estimated_max: f32) -> Normaliser {
        Normaliser {
            max_threshold: estimated_max,
        }
    }

    pub fn normalise(&mut self, bars: &mut Vec<f32>) {
        let mut max: f32 = 0.0;

        bars.iter_mut().for_each(|b| {
            max = max.max(*b);
            *b /= self.max_threshold
        });

        if max > self.max_threshold {
            self.max_threshold = max * 1.2
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::normaliser::{self, Normaliser};

    fn normalisation_test(
        input: &mut Vec<f32>,
        expected_output: &Vec<f32>,
        starting_threshold: Option<f32>,
        normaliser: Option<Normaliser>,
    ) -> Normaliser {
        let mut t = normaliser.unwrap_or_else(|| {
            Normaliser::new(
                starting_threshold
                    .expect("starting threshold must be provided if a normaliser isn't"),
            )
        });

        t.normalise(input);

        input
            .iter()
            .zip(expected_output.iter())
            .for_each(|(i, o)| assert!((i - o).abs() < 1e-5));

        t
    }

    #[test]
    fn test_normalisation_normalises_values() {
        normalisation_test(
            &mut vec![0.0_f32, 5.0, 10.0, 25.0, 49.0],
            &vec![0.0_f32, 0.1, 0.2, 0.5, 0.98],
            Some(50.0),
            None,
        );
    }

    #[test]
    fn test_normalisation_adjusts_max() {
        let n = normalisation_test(
            &mut vec![0.0_f32, 5.0, 10.0, 25.0, 49.0, 100.0],
            &vec![0.0_f32, 0.1, 0.2, 0.5, 0.98, 2.0],
            Some(50.0),
            None,
        );

        normalisation_test(
            &mut vec![0.0_f32, 12.0, 36.0, 60.0, 117.6],
            &vec![0.0_f32, 0.1, 0.3, 0.5, 0.98],
            None,
            Some(n),
        );
    }
}
