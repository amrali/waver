//   Copyright 2019 Waver Contributors
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.

//! Basic FFT spectrum analysis functionality.

use crate::Waveform;
use alloc::vec::Vec;
use core::iter::Sum;
use num_complex::Complex;
use rustfft::FftPlanner;

/// A structure that represents the frequency spectrum of a signal.
#[derive(Debug, Clone, PartialEq)]
pub struct Spectrum<F> {
    /// The frequency resolution of the spectrum.
    pub frequency_resolution: F,
    /// The frequency spectrum data, as a vector of (frequency, magnitude, phase) tuples.
    pub data: Vec<(F, F, F)>,
}

/// Performs a frequency analysis on a `Waveform`.
///
/// # Arguments
///
/// * `waveform` - The waveform to analyze.
/// * `num_samples` - The number of samples to analyze.
///
/// # Returns
///
/// The frequency spectrum of the waveform.
pub fn spectrum<BitDepth>(waveform: &Waveform<BitDepth>, num_samples: usize) -> Spectrum<BitDepth>
where
    BitDepth: Clone + rustfft::FftNum + num_traits::Float + Sum,
{
    let mut planner = FftPlanner::<BitDepth>::new();
    let fft = planner.plan_fft_forward(num_samples);

    let mut buffer: Vec<Complex<BitDepth>> = waveform
        .iter()
        .take(num_samples)
        .map(|sample| Complex::new(sample, BitDepth::zero()))
        .collect();

    fft.process(&mut buffer);

    let frequency_resolution =
        BitDepth::from(waveform.sample_rate).unwrap() / BitDepth::from(num_samples).unwrap();
    let data = buffer
        .iter()
        .take(num_samples / 2)
        .enumerate()
        .map(|(bin_index, complex_value)| {
            let frequency = BitDepth::from(bin_index).unwrap() * frequency_resolution;
            let magnitude =
                (complex_value.re * complex_value.re + complex_value.im * complex_value.im).sqrt();
            let phase = complex_value.im.atan2(complex_value.re);
            (frequency, magnitude, phase)
        })
        .collect();

    Spectrum {
        frequency_resolution,
        data,
    }
}
