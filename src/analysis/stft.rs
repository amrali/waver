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
use num_complex::Complex;
use rustfft::{FftNum, FftPlanner};

/// Performs a Short-Time Fourier Transform (STFT) on a floating-point `Waveform`.
///
/// This function analyzes a recorded waveform using overlapping windows and returns
/// a series of frequency spectra over time. It's designed to work exclusively with
/// floating-point waveforms to maintain precision throughout the analysis.
///
/// For integer samples, use the quantization module to convert to floats first:
/// ```rust
/// use waver::{Waveform, analysis::time_spectrum, quantization::dequantize_samples};
///
/// let samples: Vec<i16> = vec![100, 200, 150, 175]; // Example samples
/// let float_samples: Vec<f32> = dequantize_samples(&samples);
/// let float_waveform: Waveform<f32> = Waveform::from_recorded_samples(44100.0, &float_samples);
/// let result = time_spectrum(&float_waveform, 1024, 512);
/// ```
///
/// # Arguments
///
/// * `waveform` - A floating-point waveform to analyze
/// * `window_size` - Size of each analysis window (should be a power of 2)
/// * `hop_size` - Number of samples to advance between windows
///
/// # Returns
///
/// A `Result` containing a `Vec<Spectrum<BitDepth>>` representing the spectrogram of the waveform.
pub fn time_spectrum<BitDepth>(
    waveform: &Waveform<BitDepth>,
    window_size: usize,
    hop_size: usize,
) -> Result<Vec<Spectrum<BitDepth>>, Error>
where
    BitDepth: Clone + FftNum + num_traits::Float,
{
    let WaveformSource::Recorded(recorded_data) = &waveform.source else {
        return Err(Error::UnsupportedSource);
    };

    let mut planner = FftPlanner::<BitDepth>::new();
    let fft = planner.plan_fft_forward(window_size);

    // Pre-compute Hann window using BitDepth precision
    let hann_window: Vec<BitDepth> = (0..window_size)
        .map(|i| {
            // Use BitDepth precision throughout
            let half = BitDepth::from(0.5).unwrap();
            let one = BitDepth::one();
            let two = BitDepth::from(2.0).unwrap();
            let pi = BitDepth::from(core::f64::consts::PI).unwrap();
            let i_f = BitDepth::from(i).unwrap();
            let size_f = BitDepth::from(window_size - 1).unwrap();
            half * (one - (two * pi * i_f / size_f).cos())
        })
        .collect();

    let spectra = recorded_data
        .windows(window_size)
        .step_by(hop_size)
        .map(|chunk| {
            let mut buffer: Vec<Complex<BitDepth>> = chunk
                .iter()
                .zip(&hann_window)
                .map(|(&sample, &window_value)| {
                    let zero = BitDepth::zero();
                    Complex::new(sample * window_value, zero)
                })
                .collect();

            fft.process(&mut buffer);

            // Use BitDepth precision throughout - no casting needed
            let frequency_resolution = BitDepth::from(waveform.sample_rate).unwrap()
                / BitDepth::from(window_size).unwrap();
            let data = buffer
                .iter()
                .take(window_size / 2)
                .enumerate()
                .map(|(bin_index, complex_value)| {
                    let frequency = BitDepth::from(bin_index).unwrap() * frequency_resolution;
                    // Maintain BitDepth precision for magnitude calculation
                    let magnitude_squared =
                        complex_value.re * complex_value.re + complex_value.im * complex_value.im;
                    let magnitude = magnitude_squared.sqrt();
                    let phase = complex_value.im.atan2(complex_value.re);
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
