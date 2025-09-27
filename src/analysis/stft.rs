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

//! Short-Time Fourier Transform (STFT) implementation.

use super::spectrum::Spectrum;
use crate::{
    error::Error,
    waveform::{Waveform, WaveformSource},
};
use alloc::vec::Vec;
use core::f32::consts::PI;
use libm::atan2f;
use num_complex::Complex;
use rustfft::FftPlanner;

/// Performs a Short-Time Fourier Transform (STFT) on a `Waveform`.
///
/// # Errors
///
/// This function will return an error if the waveform is not from a recorded source.
///
/// # Arguments
///
/// * `waveform` - The waveform to analyze.
/// * `window_size` - The number of samples in each analysis window.
/// * `hop_size` - The number of samples to hop between windows.
///
/// # Returns
///
/// A `Result` containing a `Vec<Spectrum>` representing the spectrogram of the waveform.
pub fn time_spectrum<BitDepth: Clone>(
    waveform: &Waveform<BitDepth>,
    window_size: usize,
    hop_size: usize,
) -> Result<Vec<Spectrum>, Error> {
    let WaveformSource::Recorded(recorded_data) = &waveform.source else {
        return Err(Error::UnsupportedSource);
    };

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(window_size);

    // Pre-compute Hann window
    let hann_window: Vec<f32> = (0..window_size)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (window_size - 1) as f32).cos()))
        .collect();

    let spectra = recorded_data
        .windows(window_size)
        .step_by(hop_size)
        .map(|chunk| {
            let mut buffer: Vec<Complex<f32>> = chunk
                .iter()
                .zip(&hann_window)
                .map(|(&sample, &window_value)| Complex::new(sample as f32 * window_value, 0.0))
                .collect();

            fft.process(&mut buffer);

            let frequency_resolution = waveform.sample_rate / window_size as f32;
            let data = buffer
                .iter()
                .take(window_size / 2)
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
        })
        .collect();

    Ok(spectra)
}
