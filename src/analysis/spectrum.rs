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

use crate::waveform::Waveform;
use alloc::vec::Vec;
use libm::atan2f;
use num_complex::Complex;
use num_traits::AsPrimitive;
use rustfft::FftPlanner;

/// A structure that represents the frequency spectrum of a signal.
#[derive(Debug, Clone, PartialEq)]
pub struct Spectrum {
    /// The frequency resolution of the spectrum.
    pub frequency_resolution: f32,
    /// The frequency spectrum data, as a vector of (frequency, magnitude, phase) tuples.
    pub data: Vec<(f32, f32, f32)>,
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
pub fn spectrum<BitDepth: Clone + num_traits::Bounded + num_traits::NumCast + AsPrimitive<f32>>(
    waveform: &Waveform<BitDepth>,
    num_samples: usize,
) -> Spectrum {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(num_samples);

    let mut buffer: Vec<Complex<f32>> = waveform
        .iter()
        .take(num_samples)
        .map(|sample| Complex::new(sample.as_(), 0.0))
        .collect();

    fft.process(&mut buffer);

    let frequency_resolution = waveform.sample_rate / num_samples as f32;
    let data = buffer
        .iter()
        .take(num_samples / 2)
        .enumerate()
        .map(|(bin_index, complex_value)| {
            let frequency = bin_index as f32 * frequency_resolution;
            let magnitude = (complex_value.re.powi(2) + complex_value.im.powi(2)).sqrt();
            let phase = atan2f(complex_value.im, complex_value.re);
            (frequency, magnitude, phase)
        })
        .collect();

    Spectrum {
        frequency_resolution,
        data,
    }
}
