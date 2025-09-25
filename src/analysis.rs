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

//! A module for `no-std` signal analysis.
use crate::waveform::Waveform;
use alloc::vec::Vec;
use num_complex::Complex;
use num_traits::AsPrimitive;
use rustfft::FftPlanner;

/// A structure that represents the frequency spectrum of a signal.
#[derive(Debug, Clone, PartialEq)]
pub struct Spectrum {
    /// The frequency resolution of the spectrum.
    pub frequency_resolution: f32,
    /// The frequency spectrum data.
    pub data: Vec<(f32, f32)>,
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
        .map(|x| Complex::new(x.as_() as f32, 0.0))
        .collect();

    fft.process(&mut buffer);

    let frequency_resolution = waveform.sample_rate as f32 / num_samples as f32;
    let data = buffer
        .iter()
        .take(num_samples / 2)
        .enumerate()
        .map(|(i, c)| {
            let freq = i as f32 * frequency_resolution;
            let magnitude = (c.re.powi(2) + c.im.powi(2)).sqrt();
            (freq, magnitude)
        })
        .collect();

    Spectrum {
        frequency_resolution,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waveform::Waveform;
    extern crate std;
    use std::f32::consts::PI;

    #[test]
    fn test_spectrum_recorded() {
        let sample_rate = 44100.0;
        let frequency = 440.0;
        let samples: Vec<i16> = (0..sample_rate as u32)
            .map(|i| {
                let t = i as f32 / sample_rate;
                let sample = (t * frequency * 2.0 * PI).sin();
                let amplitude = i16::MAX as f32;
                (sample * amplitude) as i16
            })
            .collect();

        let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
        let spectrum = spectrum(&waveform, samples.len());

        // Find the dominant frequency.
        let (dominant_freq, _) = spectrum
            .data
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();

        // The dominant frequency should be approximately 440Hz.
        assert!((dominant_freq - frequency).abs() < spectrum.frequency_resolution);
    }

    #[test]
    fn test_spectrum_generative() {
        let sample_rate = 44100.0;
        let frequency = 440.0;
        let waveform = Waveform::<i16>::with_wave(
            sample_rate,
            crate::Wave {
                frequency,
                ..Default::default()
            },
        );
        let spectrum = spectrum(&waveform, sample_rate as usize);

        // Find the dominant frequency.
        let (dominant_freq, _) = spectrum
            .data
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();

        // The dominant frequency should be approximately 440Hz.
        assert!((dominant_freq - frequency).abs() < spectrum.frequency_resolution);
    }
}