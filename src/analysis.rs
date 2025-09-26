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

//! This module provides functions for analyzing `Waveform` instances,
//! including frequency analysis (FFT), time-based frequency analysis (STFT),
//! and waveform synthesis.
use crate::{
    error::Error,
    waveform::{Waveform, WaveformSource},
    Modulation, Wave,
};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::f32::consts::PI;
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
            let phase = atan2f(c.im, c.re);
            (freq, magnitude, phase)
        })
        .collect();

    Spectrum {
        frequency_resolution,
        data,
    }
}

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
    if let WaveformSource::Recorded(data) = &waveform.source {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(window_size);
        let hann_window: Vec<f32> = (0..window_size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (window_size - 1) as f32).cos()))
            .collect();

        let spectra = data
            .windows(window_size)
            .step_by(hop_size)
            .map(|chunk| {
                let mut buffer: Vec<Complex<f32>> = chunk
                    .iter()
                    .zip(hann_window.iter())
                    .map(|(&sample, &win)| Complex::new(sample as f32 * win, 0.0))
                    .collect();

                fft.process(&mut buffer);

                let frequency_resolution = waveform.sample_rate as f32 / window_size as f32;
                let data = buffer
                    .iter()
                    .take(window_size / 2)
                    .enumerate()
                    .map(|(i, c)| {
                        let freq = i as f32 * frequency_resolution;
                        let magnitude = (c.re.powi(2) + c.im.powi(2)).sqrt();
                        let phase = atan2f(c.im, c.re);
                        (freq, magnitude, phase)
                    })
                    .collect();

                Spectrum {
                    frequency_resolution,
                    data,
                }
            })
            .collect();
        Ok(spectra)
    } else {
        Err(Error::UnsupportedSource)
    }
}

/// Synthesizes a generative `Waveform` from a recorded `Waveform`.
///
/// # Errors
///
/// This function will return an error if the waveform is not from a recorded source.
///
/// # Arguments
///
/// * `waveform` - The recorded waveform to synthesize.
/// * `window_size` - The number of samples in each analysis window.
/// * `hop_size` - The number of samples to hop between windows.
/// * `num_harmonics` - The number of harmonics to synthesize.
///
/// # Returns
///
/// A `Result` containing a new generative `Waveform`.
pub fn synthesize<BitDepth: Clone>(
    waveform: &Waveform<BitDepth>,
    window_size: usize,
    hop_size: usize,
    num_harmonics: usize,
) -> Result<Waveform<BitDepth>, Error> {
    let spectrogram = time_spectrum(waveform, window_size, hop_size)?;
    let mut harmonic_tracks: BTreeMap<i32, (Vec<f32>, Vec<f32>)> = BTreeMap::new();

    for spectrum in spectrogram {
        let mut peaks = spectrum.data.clone();
        peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (freq, mag, phase) in peaks.iter().take(num_harmonics) {
            let freq_key = (freq.round() as i32) / 10 * 10;
            let (amplitude_envelope, phase_envelope) =
                harmonic_tracks.entry(freq_key).or_insert((Vec::new(), Vec::new()));
            amplitude_envelope.push(*mag);
            phase_envelope.push(*phase);
        }
    }

    let total_magnitude: f32 = harmonic_tracks
        .values()
        .map(|(amps, _)| amps.iter().sum::<f32>())
        .sum();

    let waves: Vec<Wave> = harmonic_tracks
        .into_iter()
        .map(|(freq, (amps, phases))| Wave {
            frequency: freq as f32,
            amplitude: Modulation::Envelope(amps.iter().map(|a| a / total_magnitude).collect()),
            phase: Modulation::Envelope(phases),
            ..Default::default()
        })
        .collect();

    let mut new_waveform = Waveform::new(waveform.sample_rate);
    for wave in waves {
        new_waveform.superpose(wave)?;
    }
    Ok(new_waveform)
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
        let (dominant_freq, _, _) = spectrum
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
        let (dominant_freq, _, _) = spectrum
            .data
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();

        // The dominant frequency should be approximately 440Hz.
        assert!((dominant_freq - frequency).abs() < spectrum.frequency_resolution);
    }

    #[test]
    fn test_time_spectrum() {
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
        let spectrogram = time_spectrum(&waveform, 1024, 512).unwrap();

        // The number of frames should be correct.
        assert_eq!(spectrogram.len(), 85);

        // The dominant frequency in each frame should be approximately 440Hz.
        for spectrum in spectrogram {
            let (dominant_freq, _, _) = spectrum
                .data
                .iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap();
            assert!((dominant_freq - frequency).abs() < spectrum.frequency_resolution);
        }
    }

    #[test]
    fn test_synthesize() {
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
        let synthesized_waveform = synthesize(&waveform, 1024, 512, 1).unwrap();

        if let WaveformSource::Generative(waves) = synthesized_waveform.source {
            assert_eq!(waves.len(), 1);
            assert!((waves[0].frequency - frequency).abs() < 50.0);
            if let Modulation::Envelope(amp_env) = &waves[0].amplitude {
                assert!(amp_env.len() > 1);
            } else {
                panic!("Expected amplitude envelope");
            }
            if let Modulation::Envelope(phase_env) = &waves[0].phase {
                assert!(phase_env.len() > 1);
            } else {
                panic!("Expected phase envelope");
            }
        } else {
            panic!("Expected generative waveform");
        }
    }

    #[test]
    fn test_time_spectrum_on_generative_error() {
        let wf = Waveform::<i16>::new(44100.0);
        let err = time_spectrum(&wf, 1024, 512).unwrap_err();
        assert_eq!(err, Error::UnsupportedSource);
    }

    #[test]
    fn test_synthesize_on_generative_error() {
        let wf = Waveform::<i16>::new(44100.0);
        let err = synthesize(&wf, 1024, 512, 1).unwrap_err();
        assert_eq!(err, Error::UnsupportedSource);
    }
}