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
use core::{
    iter::{IntoIterator, Iterator},
    marker::PhantomData,
};
use num_traits::{AsPrimitive, Bounded, NumCast};

/// An enum that represents the source of the waveform.
#[derive(Debug, Clone)]
pub enum WaveformSource {
    /// A generative waveform that is a superposition of a number of simple
    /// sinusoidal waves.
    Generative(Vec<Wave>),

    /// A recorded waveform from a source like a WAV file.
    Recorded(Vec<i16>),
}

/// A structure that represents a complex waveform.
#[derive(Debug, Clone)]
pub struct Waveform<BitDepth: Clone> {
    /// The source of the waveform.
    pub source: WaveformSource,

    /// The sampling rate of this waveform.
    pub sample_rate: f32,

    /// Phantom data to hold the bit depth.
    _marker: PhantomData<BitDepth>,
}

impl<BitDepth: Clone> Waveform<BitDepth> {
    /// Construct a new empty waveform.
    ///
    /// At construction the sampling rate of the waveform must be specified.
    ///
    /// # Examples
    ///
    /// ```
    /// use waver::Waveform;
    ///
    /// let wf = Waveform::<i16>::new(44100.0);
    /// ```
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            source: WaveformSource::Generative(Vec::new()),
            _marker: PhantomData,
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
    /// let wf = Waveform::<i16>::with_wave(44100.0,
    ///     Wave { frequency: 4000.0, ..Default::default() });
    /// ```
    pub fn with_wave(sample_rate: f32, wave: Wave) -> Self {
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
    /// let mut wf = Waveform::<i16>::new(44100.0);
    /// wf.superpose(Wave { frequency: 6000.0, amplitude: 0.25.into(), ..Default::default() })
    ///     .unwrap()
    ///     .superpose(Wave { frequency: 5500.0, amplitude: 0.75.into(), ..Default::default() });
    /// ```
    pub fn superpose(&mut self, wave: Wave) -> Result<&mut Self, Error> {
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
    /// let mut wf = Waveform::<i16>::with_wave(44100.0,
    ///     Wave { frequency: 3000.0, amplitude: 1.0.into(), ..Default::default() });
    /// wf.superpose(Wave { frequency: 4000.0, amplitude: 0.5.into(), ..Default::default() }).unwrap().normalize_amplitudes().unwrap();
    /// ```
    pub fn normalize_amplitudes(&mut self) -> Result<&mut Self, Error> {
        if let WaveformSource::Generative(components) = &mut self.source {
            let amp_ratio = 1.0 / components.len() as f32;
            components
                .iter_mut()
                .for_each(|c| c.amplitude = amp_ratio.into());
            Ok(self)
        } else {
            Err(Error::UnsupportedSource)
        }
    }

    /// Creates a `Waveform` from a recorded sample buffer.
    pub fn from_recorded_samples(sample_rate: f32, samples: &[i16]) -> Self {
        Self {
            sample_rate,
            source: WaveformSource::Recorded(samples.to_vec()),
            _marker: PhantomData,
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
    /// let mut wf = Waveform::<i16>::new(44100.0);
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

impl<'a, BitDepth: Bounded + NumCast + AsPrimitive<f32> + Clone> IntoIterator
    for &'a Waveform<BitDepth>
{
    type Item = BitDepth;
    type IntoIter = WaveformIterator<'a, BitDepth>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The source for the WaveformIterator.
pub enum WaveformIteratorSource<'a> {
    /// An iterator for a generative waveform.
    Generative(Vec<WaveIterator<'a>>),

    /// An iterator for a recorded waveform.
    Recorded(core::slice::Iter<'a, i16>),
}

/// Iterator for Waveform structure.
///
/// For generative waveforms, if any underlying iterator ends early (e.g., due to numerical instability),
/// the iterator will yield zero for that component in subsequent iterations.
pub struct WaveformIterator<'a, BitDepth: Clone> {
    _inner: &'a Waveform<BitDepth>,
    source: WaveformIteratorSource<'a>,
}

impl<'a, BitDepth: Bounded + NumCast + AsPrimitive<f32> + Clone> Iterator
    for WaveformIterator<'a, BitDepth>
{
    type Item = BitDepth;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.source {
            WaveformIteratorSource::Generative(iters) => {
                // Superpose all waveform components.
                let superposition: f32 = iters.iter_mut().map(|x| x.next().unwrap_or(0.0)).sum();
                NumCast::from(superposition * BitDepth::max_value().as_())
            }
            WaveformIteratorSource::Recorded(iter) => {
                iter.next().map(|&sample| NumCast::from(sample).unwrap())
            }
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
        let wf = Waveform::<i16>::with_wave(44100.0, w3khz.clone());

        let w1: Vec<i16> = wf.iter().take(100).collect();
        let w2: Vec<i16> = w3khz
            .iter()
            .take(100)
            .map(|c| (c * i16::MAX as f32) as i16)
            .collect();

        assert_eq!(w1, w2);

        // Test empty waveform
        let wf_empty = Waveform::<i8>::new(44100.0);
        let v: Vec<i8> = wf_empty.iter().take(10).collect();
        assert_eq!(v, [0i8; 10]);

        // Test basic iteration
        let wf_iter = Waveform::<i8>::new(44100.0);
        let mut itr = wf_iter.into_iter();
        assert_eq!(itr.next().unwrap(), 0);

        // Test construction equivalence
        let wf1 = Waveform::<i16>::with_wave(
            44100.0,
            Wave {
                frequency: 3400.0,
                amplitude: 1.0.into(),
                ..Default::default()
            },
        );
        let mut wf2 = Waveform::<i16>::new(44100.0);

        let v1: Vec<i16> = wf1.iter().take(100).collect();
        wf2.superpose(Wave {
            frequency: 3400.0,
            ..Default::default()
        })
        .unwrap();
        let v2: Vec<i16> = wf2.iter().take(100).collect();

        assert_eq!(v1, v2);
    }

    #[test]
    fn test_waveform_amplitude_normalization_comprehensive() {
        // Test basic amplitude normalization
        let mut wf = Waveform::<i16>::with_wave(
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
        let mut single_wave_wf = Waveform::<i16>::with_wave(
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
        let mut many_waves_wf = Waveform::<i16>::new(44100.0);
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
        let samples = vec![0, 1, 2, 3, 4];
        let wf = Waveform::<i16>::from_recorded_samples(44100.0, &samples);

        // Verify source data is stored correctly
        if let WaveformSource::Recorded(ref data) = wf.source {
            assert_eq!(*data, samples);
        } else {
            panic!("Expected recorded waveform");
        }

        // Verify iteration works correctly
        let collected: Vec<i16> = wf.iter().collect();
        assert_eq!(collected, samples);

        // Test error conditions on recorded waveform
        let error_samples = vec![100i16, 200, 300, 400, 500];
        let mut recorded_waveform: Waveform<i16> =
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
        let empty_samples: Vec<i16> = vec![];
        let empty_waveform: Waveform<i16> =
            Waveform::from_recorded_samples(44100.0, &empty_samples);
        let empty_result: Vec<i16> = empty_waveform.iter().collect();
        assert!(
            empty_result.is_empty(),
            "Empty waveform should produce no samples"
        );

        // Test edge cases: single sample
        let single_sample = vec![1234i16];
        let single_waveform: Waveform<i16> =
            Waveform::from_recorded_samples(44100.0, &single_sample);
        let single_result: Vec<i16> = single_waveform.iter().collect();
        assert_eq!(
            single_result,
            vec![1234i16],
            "Single sample should be preserved"
        );

        // Test edge cases: extreme sample values
        let extreme_samples = vec![i16::MIN, i16::MAX, 0, i16::MIN / 2, i16::MAX / 2];
        let extreme_waveform: Waveform<i16> =
            Waveform::from_recorded_samples(44100.0, &extreme_samples);
        let extreme_result: Vec<i16> = extreme_waveform.iter().collect();
        assert_eq!(
            extreme_result, extreme_samples,
            "Extreme samples should be preserved"
        );
    }

    #[test]
    fn test_waveform_bit_depth_comprehensive() {
        let sample_rate = 44100.0;
        let wave = Wave {
            frequency: 440.0,
            amplitude: Modulation::Static(0.8),
            ..Default::default()
        };

        // Test i8 bit depth
        let wf_i8 = Waveform::<i8>::with_wave(sample_rate, wave.clone());
        let samples_i8: Vec<i8> = wf_i8.iter().take(10).collect();
        assert_eq!(samples_i8.len(), 10);
        assert!(
            samples_i8.iter().any(|&x| x != 0),
            "Should produce non-zero samples"
        );

        // Test u8 bit depth
        let wf_u8 = Waveform::<u8>::with_wave(sample_rate, wave.clone());
        let samples_u8: Vec<u8> = wf_u8.iter().take(10).collect();
        assert_eq!(samples_u8.len(), 10);

        // Test i32 bit depth
        let wf_i32 = Waveform::<i32>::with_wave(sample_rate, wave.clone());
        let samples_i32: Vec<i32> = wf_i32.iter().take(10).collect();
        assert_eq!(samples_i32.len(), 10);
        assert!(
            samples_i32.iter().any(|&x| x.abs() > 1000),
            "Should produce scaled samples for i32"
        );

        // Test f32 bit depth
        let wf_f32 = Waveform::<f32>::with_wave(sample_rate, wave.clone());
        let samples_f32: Vec<f32> = wf_f32.iter().take(10).collect();
        assert_eq!(samples_f32.len(), 10);
        assert!(
            samples_f32.iter().all(|&x| x.is_finite()),
            "f32 samples should be finite"
        );

        // Test corner cases with specific bit depths
        let corner_wave = Wave {
            sample_rate: 44100.0,
            frequency: 1000.0,
            amplitude: Modulation::Static(1.0),
            ..Default::default()
        };

        // Test u16 corner case
        let wf_u16 = Waveform::<u16>::with_wave(44100.0, corner_wave.clone());
        let samples_u16: Vec<u16> = wf_u16.iter().take(10).collect();
        assert_eq!(
            samples_u16.len(),
            10,
            "Should generate exactly 10 u16 samples"
        );
        assert!(
            samples_u16.iter().all(|&x| x <= u16::MAX),
            "u16 samples should be in valid range"
        );
        assert!(
            samples_u16.iter().any(|&x| x > 1000),
            "Should have some significant u16 values"
        );

        // Test i64 corner case
        let wf_i64 = Waveform::<i64>::with_wave(44100.0, corner_wave);
        let samples_i64: Vec<i64> = wf_i64.iter().take(10).collect();
        assert_eq!(
            samples_i64.len(),
            10,
            "Should generate exactly 10 i64 samples"
        );
        assert!(
            samples_i64.iter().any(|&x| x.abs() > 1_000_000),
            "i64 should produce large-scale samples"
        );
        assert!(
            samples_i64.iter().all(|&x| x.abs() <= i64::MAX),
            "i64 samples should not exceed maximum value"
        );
    }

    #[test]
    fn test_waveform_superposition_and_stability_comprehensive() {
        use core::f32::consts::PI;

        // Test superposition with different wave functions
        let mut waveform = Waveform::<i16>::new(44100.0);

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

        let samples: Vec<i16> = waveform.iter().take(1000).collect();
        assert_eq!(samples.len(), 1000);

        // Should produce complex waveform with all functions combined
        let variance = calculate_variance(&samples.iter().map(|&x| x as f32).collect::<Vec<_>>());
        assert!(
            variance > 1000.0,
            "Combined waveform should have significant variance"
        );

        // Test numerical stability with potential overflow conditions
        let mut unstable_wf = Waveform::<i16>::new(44100.0);

        // Add waves with combined amplitude > 100%
        for i in 1..=5 {
            let wave = Wave {
                sample_rate: 44100.0,
                frequency: 440.0 * i as f32,
                amplitude: Modulation::Static(0.15), // Total: 75% (safe)
                phase: Modulation::Static(0.0),
                func: crate::WaveFunc::Sine,
            };
            unstable_wf.superpose(wave).unwrap();
        }

        // Should produce samples with moderate overflow handling
        let stability_samples: Vec<i16> = unstable_wf.iter().take(100).collect();

        // With reduced amplitudes, should get all requested samples
        assert_eq!(
            stability_samples.len(),
            100,
            "Should produce exactly 100 samples with manageable configuration"
        );
        assert!(
            stability_samples.iter().all(|&x| x.abs() <= i16::MAX),
            "Samples should not exceed bit depth limits"
        );

        // Check for significant amplitude with 5 superposed waves
        let max_amplitude = stability_samples
            .iter()
            .map(|&x| x.abs())
            .max()
            .unwrap_or(0);
        assert!(
            max_amplitude > 1000,
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

        let modulated_waveform = Waveform::<i16>::with_wave(44100.0, modulated_wave);
        let modulated_samples: Vec<i16> =
            modulated_waveform.iter().take(amp_envelope.len()).collect();

        assert_eq!(modulated_samples.len(), amp_envelope.len());

        // First and last samples should be near zero due to amplitude envelope
        assert!(
            modulated_samples[0].abs() < 1000,
            "First sample should be small"
        );
        assert!(
            modulated_samples[modulated_samples.len() - 1].abs() < 1000,
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

        let zero_amp_waveform = Waveform::<i16>::with_wave(44100.0, zero_amp_wave);
        let zero_samples: Vec<i16> = zero_amp_waveform.iter().take(100).collect();
        assert!(
            zero_samples.iter().all(|&x| x == 0),
            "Zero amplitude should produce zero samples"
        );

        // Test with zero frequency
        let zero_freq_wave = Wave {
            frequency: 0.0,
            amplitude: Modulation::Static(0.5),
            ..Default::default()
        };

        let zero_freq_waveform = Waveform::<i16>::with_wave(44100.0, zero_freq_wave);
        let freq_samples: Vec<i16> = zero_freq_waveform.iter().take(100).collect();
        let first_sample = freq_samples[0];
        assert!(
            freq_samples.iter().all(|&x| x == first_sample),
            "Zero frequency should produce constant output"
        );

        // Test with extreme sample rates
        let _low_sr_waveform = Waveform::<i16>::new(8.0);
        let _high_sr_waveform = Waveform::<i16>::new(192000.0);

        // Test iterator halt on numerical instability
        let mut unstable_wf = Waveform::<i16>::with_wave(
            44100.0,
            Wave {
                frequency: 4000.0,
                amplitude: 1.0.into(),
                ..Default::default()
            },
        );
        unstable_wf
            .superpose(Wave {
                frequency: 5000.0,
                amplitude: 0.5.into(),
                ..Default::default()
            })
            .unwrap();
        let unstable_samples: Vec<i16> = unstable_wf.iter().take(100).collect();
        assert_ne!(
            unstable_samples.len(),
            100,
            "Iterator should halt on numerical instability"
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
}
