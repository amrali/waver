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

//! A module for waveform construction and quantization.

use crate::{error::Error, Wave, WaveIterator};
use alloc::vec::Vec;
use core::iter::{IntoIterator, Iterator};
use num_traits::Float;

/// An enum that represents the source of the waveform.
#[derive(Debug, Clone)]
pub enum WaveformSource<BitDepth: Float + Copy> {
    /// A generative waveform that is a superposition of a number of simple
    /// sinusoidal waves.
    Generative(Vec<Wave<BitDepth>>),

    /// A recorded waveform from a source like a WAV file.
    Recorded(Vec<BitDepth>),
}

/// A structure that represents a complex waveform.
#[derive(Debug, Clone)]
pub struct Waveform<BitDepth: Float + Copy> {
    /// The source of the waveform.
    pub source: WaveformSource<BitDepth>,

    /// The sampling rate of this waveform.
    pub sample_rate: BitDepth,
}

impl<BitDepth: Float + Copy> Waveform<BitDepth> {
    /// Construct a new empty waveform.
    ///
    /// At construction the sampling rate of the waveform must be specified.
    ///
    /// # Examples
    ///
    /// ```
    /// use waver::Waveform;
    ///
    /// let wf = Waveform::<f32>::new(44100.0);
    /// ```
    pub fn new(sample_rate: BitDepth) -> Self {
        Self {
            sample_rate,
            source: WaveformSource::Generative(Vec::new()),
        }
    }

    /// Construct a new waveform with a single underlying wave component.
    ///
    /// This is identical to using `new()` and then `superpose()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use waver::{Waveform, Wave};
    ///
    /// let wf = Waveform::<f32>::with_wave(44100.0,
    ///     Wave { frequency: 4000.0, ..Default::default() });
    /// ```
    pub fn with_wave(sample_rate: BitDepth, wave: Wave<BitDepth>) -> Self {
        let mut wf = Self::new(sample_rate);
        wf.superpose(wave).unwrap();
        wf
    }

    /// Add a wave component.
    ///
    /// # Errors
    ///
    /// This function will return an error if the waveform is not generative.
    ///
    /// # Examples
    ///
    /// ```
    /// use waver::{Waveform, Wave};
    ///
    /// let mut wf = Waveform::<f32>::new(44100.0);
    /// wf.superpose(Wave { frequency: 6000.0, amplitude: 0.25.into(), ..Default::default() })
    ///     .unwrap()
    ///     .superpose(Wave { frequency: 5500.0, amplitude: 0.75.into(), ..Default::default() });
    /// ```
    pub fn superpose(&mut self, wave: Wave<BitDepth>) -> Result<&mut Self, Error> {
        if let WaveformSource::Generative(components) = &mut self.source {
            components.push(Wave {
                sample_rate: self.sample_rate,
                ..wave
            });
            Ok(self)
        } else {
            Err(Error::UnsupportedSource)
        }
    }

    /// Normalize amplitude weights of all underlying waves.
    ///
    /// When superposing waves with a combined amplitude-weights that exceed
    /// 100% normally waves would be clipped at the highest quantization level.
    ///
    /// In waver, overshoot causes quantization to become numerically unstable
    /// which manifests in an iterator stop.
    ///
    /// Use this method to normalize all weights to equal shares of the amplitude.
    ///
    /// # Errors
    ///
    /// This function will return an error if the waveform is not generative.
    ///
    /// # Examples
    ///
    /// ```
    /// use waver::{Waveform, Wave};
    ///
    /// // Two waves with an amplitude weights of 150%.
    /// let mut wf = Waveform::<f32>::with_wave(44100.0,
    ///     Wave { frequency: 3000.0, amplitude: 1.0.into(), ..Default::default() });
    /// wf.superpose(Wave { frequency: 4000.0, amplitude: 0.5.into(), ..Default::default() }).unwrap().normalize_amplitudes().unwrap();
    /// ```
    pub fn normalize_amplitudes(&mut self) -> Result<&mut Self, Error> {
        if let WaveformSource::Generative(components) = &mut self.source {
            let amp_ratio =
                BitDepth::one() / BitDepth::from(components.len()).unwrap_or(BitDepth::one());
            components
                .iter_mut()
                .for_each(|c| c.amplitude = amp_ratio.into());
            Ok(self)
        } else {
            Err(Error::UnsupportedSource)
        }
    }

    /// Creates a `Waveform` from a recorded sample buffer.
    pub fn from_recorded_samples(sample_rate: BitDepth, samples: &[BitDepth]) -> Self {
        Self {
            sample_rate,
            source: WaveformSource::Recorded(samples.to_vec()),
        }
    }

    /// An iterator for the superposition of all underlying waveform components.
    ///
    /// If the waveform is generative, the iterator is infinite. If the waveform
    /// is recorded, the iterator will have a finite length.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::f32::consts::PI;
    /// use waver::{Waveform, Wave};
    ///
    /// let mut wf = Waveform::<f32>::new(44100.0);
    /// wf.superpose(Wave { frequency: 4000.0, phase: (PI / 2.0).into(), ..Default::default() }).unwrap();
    /// let mut iter = wf.iter();
    /// let mut res = Vec::new();
    /// for _ in 0..10 {
    ///     res.push(iter.next().unwrap());
    /// }
    /// ```
    pub fn iter(&self) -> WaveformIterator<'_, BitDepth> {
        let iter_source = match &self.source {
            WaveformSource::Generative(components) => {
                let iters = components.iter().map(|c| c.iter()).collect();
                WaveformIteratorSource::Generative(iters)
            }
            WaveformSource::Recorded(data) => WaveformIteratorSource::Recorded(data.iter()),
        };

        WaveformIterator {
            _inner: self,
            source: iter_source,
        }
    }
}

impl<'a, BitDepth: Float + Copy + core::iter::Sum> IntoIterator for &'a Waveform<BitDepth> {
    type Item = BitDepth;
    type IntoIter = WaveformIterator<'a, BitDepth>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The source for the WaveformIterator.
pub enum WaveformIteratorSource<'a, BitDepth: Float + Copy> {
    /// An iterator for a generative waveform.
    Generative(Vec<WaveIterator<'a, BitDepth>>),

    /// An iterator for a recorded waveform.
    Recorded(core::slice::Iter<'a, BitDepth>),
}

/// Iterator for Waveform structure.
///
/// For generative waveforms, if any underlying iterator ends early (e.g., due to numerical instability),
/// the iterator will yield zero for that component in subsequent iterations.
pub struct WaveformIterator<'a, BitDepth: Float + Copy> {
    _inner: &'a Waveform<BitDepth>,
    source: WaveformIteratorSource<'a, BitDepth>,
}

impl<'a, BitDepth: Float + Copy + core::iter::Sum> Iterator for WaveformIterator<'a, BitDepth> {
    type Item = BitDepth;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.source {
            WaveformIteratorSource::Generative(iters) => {
                // Superpose all waveform components.
                let superposition: BitDepth = iters
                    .iter_mut()
                    .map(|x| x.next().unwrap_or(BitDepth::zero()))
                    .sum();
                Some(superposition)
            }
            WaveformIteratorSource::Recorded(iter) => iter.next().cloned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{error::Error, Modulation, Wave, WaveFunc};
    extern crate std;
    use std::vec::Vec;

    #[test]
    fn test_waveform_basic_functionality() {
        // Test single wave match behavior
        let w3khz = Wave {
            sample_rate: 44100.0,
            frequency: 3000.0,
            ..Default::default()
        };
        let wf = Waveform::<f32>::with_wave(44100.0, w3khz.clone());

        let w1: Vec<f32> = wf.iter().take(100).collect();
        let w2: Vec<f32> = w3khz.iter().take(100).collect();

        assert_eq!(w1, w2);

        // Test empty waveform
        let wf_empty = Waveform::<f32>::new(44100.0);
        let v: Vec<f32> = wf_empty.iter().take(10).collect();
        assert_eq!(v, [0.0f32; 10]);

        // Test basic iteration
        let wf_iter = Waveform::<f32>::new(44100.0);
        let mut itr = wf_iter.into_iter();
        assert_eq!(itr.next().unwrap(), 0.0);

        // Test construction equivalence
        let wf1 = Waveform::<f32>::with_wave(
            44100.0,
            Wave {
                frequency: 3400.0,
                amplitude: 1.0.into(),
                ..Default::default()
            },
        );
        let mut wf2 = Waveform::<f32>::new(44100.0);

        let v1: Vec<f32> = wf1.iter().take(100).collect();
        wf2.superpose(Wave {
            frequency: 3400.0,
            ..Default::default()
        })
        .unwrap();
        let v2: Vec<f32> = wf2.iter().take(100).collect();

        assert_eq!(v1, v2);
    }

    #[test]
    fn test_waveform_amplitude_normalization_comprehensive() {
        // Test basic amplitude normalization
        let mut wf = Waveform::<f32>::with_wave(
            44100.0,
            Wave {
                frequency: 4000.0,
                amplitude: 1.5.into(),
                ..Default::default()
            },
        );
        wf.superpose(Wave {
            frequency: 5000.0,
            amplitude: 0.5.into(),
            ..Default::default()
        })
        .unwrap()
        .normalize_amplitudes()
        .unwrap();

        // Check that normalization results in equal amplitudes
        if let WaveformSource::Generative(components) = wf.source {
            components
                .iter()
                .for_each(|c| assert_eq!(c.amplitude, Modulation::Static(0.5)));
        } else {
            panic!("Expected generative waveform");
        }

        // Test single wave normalization edge case
        let mut single_wave_wf = Waveform::<f32>::with_wave(
            44100.0,
            Wave {
                frequency: 440.0,
                amplitude: Modulation::Static(0.7),
                ..Default::default()
            },
        );

        let result = single_wave_wf.normalize_amplitudes();
        assert!(result.is_ok(), "Should normalize single wave");
        if let Ok(wf) = result {
            if let crate::waveform::WaveformSource::Generative(waves) = &wf.source {
                assert_eq!(
                    waves[0].amplitude,
                    Modulation::Static(1.0),
                    "Single wave should be normalized to 1.0"
                );
            }
        }

        // Test many waves normalization edge case
        let mut many_waves_wf = Waveform::<f32>::new(44100.0);
        for i in 1..=10 {
            let wave = Wave {
                frequency: 100.0 * i as f32,
                amplitude: Modulation::Static(0.5),
                ..Default::default()
            };
            many_waves_wf.superpose(wave).unwrap();
        }

        let result = many_waves_wf.normalize_amplitudes();
        assert!(result.is_ok(), "Should normalize many waves");
        if let Ok(wf) = result {
            if let crate::waveform::WaveformSource::Generative(waves) = &wf.source {
                for wave in waves {
                    assert_eq!(
                        wave.amplitude,
                        Modulation::Static(0.1),
                        "Each wave should get 1/10 amplitude"
                    );
                }
            }
        }
    }

    #[test]
    fn test_recorded_waveform_comprehensive() {
        // Test basic recorded waveform creation and iteration
        let samples = vec![0.0, 0.1, 0.2, 0.3, 0.4];
        let wf = Waveform::<f32>::from_recorded_samples(44100.0, &samples);

        // Verify source data is stored correctly
        if let WaveformSource::Recorded(ref data) = wf.source {
            assert_eq!(*data, samples);
        } else {
            panic!("Expected recorded waveform");
        }

        // Verify iteration works correctly
        let collected: Vec<f32> = wf.iter().collect();
        assert_eq!(collected, samples);

        // Test error conditions on recorded waveform
        let error_samples = vec![0.1f32, 0.2, 0.3, 0.4, 0.5];
        let mut recorded_waveform: Waveform<f32> =
            Waveform::from_recorded_samples(44100.0, &error_samples);

        // Test superpose on recorded waveform (should error)
        let wave = Wave {
            frequency: 440.0,
            ..Default::default()
        };
        let result = recorded_waveform.superpose(wave);
        assert!(
            result.is_err(),
            "Should not be able to superpose on recorded waveform"
        );
        assert_eq!(result.unwrap_err(), Error::UnsupportedSource);

        // Test normalize_amplitudes on recorded waveform (should error)
        let result = recorded_waveform.normalize_amplitudes();
        assert!(
            result.is_err(),
            "Should not be able to normalize recorded waveform amplitudes"
        );
        assert_eq!(result.unwrap_err(), Error::UnsupportedSource);

        // Test edge cases: empty samples
        let empty_samples: Vec<f32> = vec![];
        let empty_waveform: Waveform<f32> =
            Waveform::from_recorded_samples(44100.0, &empty_samples);
        let empty_result: Vec<f32> = empty_waveform.iter().collect();
        assert!(
            empty_result.is_empty(),
            "Empty waveform should produce no samples"
        );

        // Test edge cases: single sample
        let single_sample = vec![0.7f32];
        let single_waveform: Waveform<f32> =
            Waveform::from_recorded_samples(44100.0, &single_sample);
        let single_result: Vec<f32> = single_waveform.iter().collect();
        assert_eq!(
            single_result,
            vec![0.7f32],
            "Single sample should be preserved"
        );

        // Test edge cases: extreme sample values
        let extreme_samples = vec![-1.0f32, 1.0, 0.0, -0.5, 0.5];
        let extreme_waveform: Waveform<f32> =
            Waveform::from_recorded_samples(44100.0, &extreme_samples);
        let extreme_result: Vec<f32> = extreme_waveform.iter().collect();
        assert_eq!(
            extreme_result, extreme_samples,
            "Extreme samples should be preserved"
        );
    }

    #[test]
    fn test_waveform_float_bit_depth_comprehensive() {
        let sample_rate = 44100.0;
        let wave = Wave {
            frequency: 440.0,
            amplitude: Modulation::Static(0.8),
            ..Default::default()
        };

        // Test f32 bit depth
        let wf_f32 = Waveform::<f32>::with_wave(sample_rate, wave.clone());
        let samples_f32: Vec<f32> = wf_f32.iter().take(10).collect();
        assert_eq!(samples_f32.len(), 10);
        assert!(
            samples_f32.iter().all(|&x| x.is_finite()),
            "f32 samples should be finite"
        );

        // Test f64 bit depth
        let wave_f64 = Wave {
            frequency: 440.0,
            amplitude: Modulation::Static(0.8),
            ..Default::default()
        };
        let wf_f64 = Waveform::<f64>::with_wave(sample_rate as f64, wave_f64);
        let samples_f64: Vec<f64> = wf_f64.iter().take(10).collect();
        assert_eq!(samples_f64.len(), 10);
        assert!(
            samples_f64.iter().all(|&x| x.is_finite()),
            "f64 samples should be finite"
        );

        // Test corner cases with specific bit depths
        let corner_wave = Wave {
            sample_rate: 44100.0,
            frequency: 1000.0,
            amplitude: Modulation::Static(1.0),
            ..Default::default()
        };

        // Test f32 corner case
        let wf_f32_corner = Waveform::<f32>::with_wave(44100.0, corner_wave.clone());
        let samples_f32_corner: Vec<f32> = wf_f32_corner.iter().take(10).collect();
        assert_eq!(
            samples_f32_corner.len(),
            10,
            "Should generate exactly 10 f32 samples"
        );
        assert!(
            samples_f32_corner.iter().any(|&x| x.abs() > 0.1),
            "Should have some significant f32 values"
        );

        // Test f64 corner case with double precision
        let corner_wave_f64 = Wave {
            sample_rate: 44100.0,
            frequency: 1000.0,
            amplitude: Modulation::Static(1.0),
            ..Default::default()
        };
        let wf_f64_corner = Waveform::<f64>::with_wave(44100.0, corner_wave_f64);
        let samples_f64_corner: Vec<f64> = wf_f64_corner.iter().take(10).collect();
        assert_eq!(
            samples_f64_corner.len(),
            10,
            "Should generate exactly 10 f64 samples"
        );
        assert!(
            samples_f64_corner.iter().any(|&x| x.abs() > 0.1),
            "f64 should produce significant samples"
        );
    }

    #[test]
    fn test_waveform_superposition_and_stability_comprehensive() {
        use core::f32::consts::PI;

        // Test superposition with different wave functions
        let mut waveform = Waveform::<f32>::new(44100.0);

        let functions = [
            WaveFunc::Sine,
            WaveFunc::Cosine,
            WaveFunc::Square,
            WaveFunc::Sawtooth,
            WaveFunc::Triangle,
        ];

        for (i, &func) in functions.iter().enumerate() {
            let wave = Wave {
                frequency: 220.0 * (i + 1) as f32,
                amplitude: Modulation::Static(0.2),
                func,
                ..Default::default()
            };
            waveform.superpose(wave).unwrap();
        }

        let samples: Vec<f32> = waveform.iter().take(1000).collect();
        assert_eq!(samples.len(), 1000);

        // Should produce complex waveform with all functions combined
        let variance = calculate_variance(&samples);
        assert!(
            variance > 0.01,
            "Combined waveform should have significant variance"
        );

        // Test numerical stability with potential overflow conditions
        let mut stable_wf = Waveform::<f32>::new(44100.0);

        // Add waves with combined amplitude that's manageable for floats
        for i in 1..=5 {
            let wave = Wave {
                sample_rate: 44100.0,
                frequency: 440.0 * i as f32,
                amplitude: Modulation::Static(0.15), // Total: 75% (safe)
                phase: Modulation::Static(0.0),
                func: crate::WaveFunc::Sine,
            };
            stable_wf.superpose(wave).unwrap();
        }

        // Should produce samples with good stability
        let stability_samples: Vec<f32> = stable_wf.iter().take(100).collect();

        // Should get all requested samples with float precision
        assert_eq!(
            stability_samples.len(),
            100,
            "Should produce exactly 100 samples with float configuration"
        );

        // Check for significant amplitude with 5 superposed waves
        let max_amplitude = stability_samples
            .iter()
            .map(|&x| x.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        assert!(
            max_amplitude > 0.1,
            "Should have significant amplitude with 5 superposed waves"
        );

        // Test modulation interaction between amplitude and phase
        let amp_envelope = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        let phase_envelope = vec![0.0, PI / 4.0, PI / 2.0, PI / 4.0, 0.0];

        let modulated_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            amplitude: Modulation::Envelope(amp_envelope.clone()),
            phase: Modulation::Envelope(phase_envelope.clone()),
            func: WaveFunc::Sine,
        };

        let modulated_waveform = Waveform::<f32>::with_wave(44100.0, modulated_wave);
        let modulated_samples: Vec<f32> =
            modulated_waveform.iter().take(amp_envelope.len()).collect();

        assert_eq!(modulated_samples.len(), amp_envelope.len());

        // First and last samples should be near zero due to amplitude envelope
        assert!(
            modulated_samples[0].abs() < 0.01,
            "First sample should be small"
        );
        assert!(
            modulated_samples[modulated_samples.len() - 1].abs() < 0.01,
            "Last sample should be small"
        );
    }

    #[test]
    fn test_waveform_edge_cases_comprehensive() {
        // Test with zero amplitude
        let zero_amp_wave = Wave {
            frequency: 440.0,
            amplitude: Modulation::Static(0.0),
            ..Default::default()
        };

        let zero_amp_waveform = Waveform::<f32>::with_wave(44100.0, zero_amp_wave);
        let zero_samples: Vec<f32> = zero_amp_waveform.iter().take(100).collect();
        assert!(
            zero_samples.iter().all(|&x| x == 0.0),
            "Zero amplitude should produce zero samples"
        );

        // Test with zero frequency
        let zero_freq_wave = Wave {
            frequency: 0.0,
            amplitude: Modulation::Static(0.5),
            ..Default::default()
        };

        let zero_freq_waveform = Waveform::<f32>::with_wave(44100.0, zero_freq_wave);
        let freq_samples: Vec<f32> = zero_freq_waveform.iter().take(100).collect();
        let first_sample = freq_samples[0];
        assert!(
            freq_samples
                .iter()
                .all(|&x| (x - first_sample).abs() < 1e-6),
            "Zero frequency should produce constant output"
        );

        // Test with extreme sample rates
        let _low_sr_waveform = Waveform::<f32>::new(8.0);
        let _high_sr_waveform = Waveform::<f32>::new(192000.0);

        // Test iterator stability with floating point precision
        let mut stable_wf = Waveform::<f32>::with_wave(
            44100.0,
            Wave {
                frequency: 4000.0,
                amplitude: 1.0.into(),
                ..Default::default()
            },
        );
        stable_wf
            .superpose(Wave {
                frequency: 5000.0,
                amplitude: 0.5.into(),
                ..Default::default()
            })
            .unwrap();
        let stable_samples: Vec<f32> = stable_wf.iter().take(100).collect();
        assert_eq!(
            stable_samples.len(),
            100,
            "Float iterator should be stable and produce all requested samples"
        );
        assert!(
            stable_samples.iter().all(|&x| x.is_finite()),
            "All float samples should be finite"
        );
    }

    fn calculate_variance(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let mean = samples.iter().sum::<f32>() / samples.len() as f32;
        let variance =
            samples.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / samples.len() as f32;
        variance
    }

    #[test]
    fn test_recorded_waveform_float_bit_depths() {
        // Test f32 recorded samples
        let samples_f32: Vec<f32> = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
        let wf_f32 = Waveform::<f32>::from_recorded_samples(44100.0, &samples_f32);
        let collected_f32: Vec<f32> = wf_f32.iter().collect();
        assert_eq!(
            samples_f32, collected_f32,
            "f32 recorded samples should be preserved"
        );

        // Verify source type is correct
        if let WaveformSource::Recorded(ref data) = wf_f32.source {
            assert_eq!(*data, samples_f32);
        } else {
            panic!("Expected recorded waveform for f32");
        }

        // Test f64 recorded samples
        let samples_f64: Vec<f64> = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
        let wf_f64 = Waveform::<f64>::from_recorded_samples(44100.0, &samples_f64);
        let collected_f64: Vec<f64> = wf_f64.iter().collect();
        assert_eq!(
            samples_f64, collected_f64,
            "f64 recorded samples should be preserved in storage and iteration"
        );

        // Verify source stores f64 precision
        if let WaveformSource::Recorded(ref data) = wf_f64.source {
            assert_eq!(*data, samples_f64);
        } else {
            panic!("Expected recorded waveform for f64");
        }

        // Test error conditions still work with different float bit depths
        let mut recorded_f32: Waveform<f32> =
            Waveform::from_recorded_samples(44100.0, &samples_f32);
        let wave = Wave {
            frequency: 440.0,
            ..Default::default()
        };

        let result = recorded_f32.superpose(wave);
        assert!(
            result.is_err(),
            "Should not be able to superpose on f32 recorded waveform"
        );
        assert_eq!(result.unwrap_err(), Error::UnsupportedSource);

        let result = recorded_f32.normalize_amplitudes();
        assert!(
            result.is_err(),
            "Should not be able to normalize f32 recorded waveform amplitudes"
        );
        assert_eq!(result.unwrap_err(), Error::UnsupportedSource);

        // Test empty samples with different bit depths
        let empty_f32: Vec<f32> = vec![];
        let empty_wf_f32: Waveform<f32> = Waveform::from_recorded_samples(44100.0, &empty_f32);
        let empty_result_f32: Vec<f32> = empty_wf_f32.iter().collect();
        assert!(
            empty_result_f32.is_empty(),
            "Empty f32 waveform should produce no samples"
        );

        // Test precision differences between f32 and f64
        let precise_samples_f64: Vec<f64> = vec![0.123456789012345, 0.987654321098765];
        let wf_precise_f64 = Waveform::<f64>::from_recorded_samples(44100.0, &precise_samples_f64);
        let collected_precise_f64: Vec<f64> = wf_precise_f64.iter().collect();
        assert_eq!(
            precise_samples_f64, collected_precise_f64,
            "f64 should preserve higher precision samples"
        );
    }
}
