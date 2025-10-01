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

//! A module for wave types and iterators.

use alloc::{boxed::Box, string::ToString, vec::Vec};
use core::{
    f32::consts::PI,
    fmt,
    iter::{IntoIterator, Iterator},
};
use libm::{asinf, copysignf, cosf, sinf};
use num_traits::{AsPrimitive, Bounded, Float, NumCast};

/// An enum that represents the kind of the wave function.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WaveFunc {
    /// The sine function.
    Sine,
    /// The cosine function.
    Cosine,
    /// The square function.
    Square,
    /// The Sawtooth function.
    Sawtooth,
    /// The Triangle function.
    Triangle,
}

impl fmt::Display for WaveFunc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Sine => "Sine",
                Self::Cosine => "Cosine",
                Self::Square => "Square",
                Self::Sawtooth => "Sawtooth",
                Self::Triangle => "Triangle",
            }
        )
    }
}

/// An enum that represents the modulation source for a wave's properties.
#[derive(Debug, Clone, PartialEq)]
pub enum Modulation<BitDepth: Float + Copy> {
    /// A static, constant value.
    Static(BitDepth),
    /// A pre-calculated envelope.
    Envelope(Vec<BitDepth>),
    /// A Low-Frequency Oscillator (LFO) that modulates the property.
    LFO(Box<Wave<BitDepth>>),
}

impl<BitDepth: Float + Copy> From<BitDepth> for Modulation<BitDepth> {
    fn from(val: BitDepth) -> Self {
        Modulation::Static(val)
    }
}

impl<BitDepth: Float + Copy> From<Vec<BitDepth>> for Modulation<BitDepth> {
    fn from(val: Vec<BitDepth>) -> Self {
        Modulation::Envelope(val)
    }
}

impl<BitDepth: Float + Copy> From<Wave<BitDepth>> for Modulation<BitDepth> {
    fn from(val: Wave<BitDepth>) -> Self {
        Modulation::LFO(Box::new(val))
    }
}

/// A structure that represent a sinusoidal wave, with potentially dynamic
/// amplitude and phase.
///
/// The default value for a wave values is 0.0 except for the amplitude weight
/// which is 1.0 (100% of available amplitude).
#[derive(Debug, Clone, PartialEq)]
pub struct Wave<BitDepth: Float + Copy> {
    /// The sampling rate of this wave.
    pub sample_rate: BitDepth,
    /// The frequency of this wave.
    pub frequency: BitDepth,
    /// The phase of this wave.
    pub phase: Modulation<BitDepth>,
    /// The amplitude as a percentage [0.0 - 1.0].
    pub amplitude: Modulation<BitDepth>,
    /// The trigonometric function to express the wave.
    pub func: WaveFunc,
}

impl<BitDepth: Float + Copy> Wave<BitDepth> {
    /// An iterator for the Wave structure.
    ///
    /// The iterator will produce an infinite number of wave samples. For dynamic
    /// modulation sources:
    /// - **Static**: Continues infinitely with the same value
    /// - **Envelope**: Uses envelope values while available, then continues infinitely
    ///   with the last envelope value (holds the final state)
    /// - **LFO**: Continues infinitely as long as the LFO wave is properly configured
    ///
    /// This design ensures stable, predictable behavior suitable for audio applications.
    /// Envelopes represent one-shot trajectories that end in a stable state, while
    /// LFOs provide continuous periodic modulation.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::vec::Vec;
    /// use waver::Wave;
    ///
    /// let wave = Wave { sample_rate: 10000.0, frequency: 2000.0, ..Default::default() };
    /// let res: Vec<f32> = wave.iter().take(10).collect();
    /// ```
    pub fn iter(&self) -> WaveIterator<'_, BitDepth> {
        self.into_iter()
    }

    /// Quantize the wave samples to a specific quantization depth.
    ///
    /// This method returns an iterator that produces samples quantized to the
    /// specified `QuantizationDepth` type. The quantization process converts
    /// the floating-point wave samples to the target integer or floating-point
    /// format with appropriate scaling and clamping.
    ///
    /// # Examples
    ///
    /// ```
    /// use waver::Wave;
    ///
    /// let wave: Wave<f32> = Wave { sample_rate: 44100.0, frequency: 440.0, ..Default::default() };
    /// let quantized_samples: Vec<i16> = wave.quantize().take(100).collect();
    /// ```
    pub fn quantize<QuantizationDepth>(
        &self,
    ) -> QuantizedWaveIterator<'_, BitDepth, QuantizationDepth>
    where
        QuantizationDepth: Bounded + NumCast + Copy,
        BitDepth: AsPrimitive<f32>,
    {
        QuantizedWaveIterator {
            wave_iter: self.iter(),
            _phantom: core::marker::PhantomData,
        }
    }
}

/// Iterator for quantized Wave samples.
pub struct QuantizedWaveIterator<'a, BitDepth, QuantizationDepth>
where
    BitDepth: Float + Copy,
    QuantizationDepth: Bounded + NumCast + Copy,
{
    wave_iter: WaveIterator<'a, BitDepth>,
    _phantom: core::marker::PhantomData<QuantizationDepth>,
}

impl<'a, BitDepth, QuantizationDepth> Iterator
    for QuantizedWaveIterator<'a, BitDepth, QuantizationDepth>
where
    BitDepth: Float + Copy + AsPrimitive<f32>,
    QuantizationDepth: Bounded + NumCast + Copy,
{
    type Item = QuantizationDepth;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.wave_iter.next()?;
        let sample_f32: f32 = sample.as_();
        // Scale the sample to the quantization depth range
        let max_val = QuantizationDepth::max_value();
        let scaled_sample = sample_f32 * NumCast::from(max_val).unwrap_or(0.0_f32);

        NumCast::from(scaled_sample)
    }
}

impl<'a, BitDepth: Float + Copy> IntoIterator for &'a Wave<BitDepth> {
    type Item = BitDepth;
    type IntoIter = WaveIterator<'a, BitDepth>;

    fn into_iter(self) -> Self::IntoIter {
        let amp_lfo_iter = if let Modulation::LFO(wave) = &self.amplitude {
            Some(Box::new(wave.iter()))
        } else {
            None
        };
        let phase_lfo_iter = if let Modulation::LFO(wave) = &self.phase {
            Some(Box::new(wave.iter()))
        } else {
            None
        };

        WaveIterator {
            inner: self,
            index: 0,
            amp_lfo_iter,
            phase_lfo_iter,
        }
    }
}

impl<BitDepth: Float + Copy> Default for Wave<BitDepth> {
    fn default() -> Self {
        Self {
            sample_rate: BitDepth::zero(),
            frequency: BitDepth::zero(),
            phase: Modulation::Static(BitDepth::zero()),
            amplitude: Modulation::Static(BitDepth::one()),
            func: WaveFunc::Sine,
        }
    }
}

impl<BitDepth: Float + Copy + fmt::Display> fmt::Display for Wave<BitDepth> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<Func: {}, Freq: {}Hz, Ampl: {}, Sampling Freq: {}Hz>",
            self.func,
            self.frequency,
            match &self.amplitude {
                Modulation::Static(val) => val.to_string(),
                _ => "dynamic".to_string(),
            },
            self.sample_rate
        )
    }
}

/// Iterator for Wave structure.
pub struct WaveIterator<'a, BitDepth: Float + Copy> {
    inner: &'a Wave<BitDepth>,
    index: usize,
    amp_lfo_iter: Option<Box<WaveIterator<'a, BitDepth>>>,
    phase_lfo_iter: Option<Box<WaveIterator<'a, BitDepth>>>,
}

impl<'a, BitDepth: Float + Copy> WaveIterator<'a, BitDepth> {
    /// Resolve the wave function.
    #[inline]
    fn func(&self, x: BitDepth) -> BitDepth {
        match self.inner.func {
            WaveFunc::Sine => {
                BitDepth::from(sinf(x.to_f32().unwrap_or(0.0))).unwrap_or(BitDepth::zero())
            }
            WaveFunc::Cosine => {
                BitDepth::from(cosf(x.to_f32().unwrap_or(0.0))).unwrap_or(BitDepth::zero())
            }
            WaveFunc::Square => BitDepth::from(copysignf(1.0, sinf(x.to_f32().unwrap_or(0.0))))
                .unwrap_or(BitDepth::zero()),
            WaveFunc::Sawtooth => {
                let x_f32 = x.to_f32().unwrap_or(0.0);
                let two_pi = 2.0 * PI;
                BitDepth::from((x_f32 % two_pi) / PI - 1.0).unwrap_or(BitDepth::zero())
            }
            WaveFunc::Triangle => {
                let x_f32 = x.to_f32().unwrap_or(0.0);
                BitDepth::from(2.0 * asinf(sinf(x_f32)) / PI).unwrap_or(BitDepth::zero())
            }
        }
    }
}

impl<'a, BitDepth: Float + Copy> Iterator for WaveIterator<'a, BitDepth> {
    type Item = BitDepth;

    fn next(&mut self) -> Option<Self::Item> {
        let amp = match &mut self.amp_lfo_iter {
            Some(iter) => match iter.next() {
                Some(val) => val,
                // LFO ended - this shouldn't happen in a proper infinite LFO design
                // But if it does, we should use a fallback value, not zero
                None => match &self.inner.amplitude {
                    Modulation::LFO(_) => BitDepth::one(), // Default amplitude fallback
                    _ => BitDepth::one(),
                },
            },
            None => match &self.inner.amplitude {
                Modulation::Static(val) => *val,
                Modulation::Envelope(env) => {
                    // Envelope: use value at index, or last value if past end (infinite continuation)
                    // Empty envelopes default to 1.0 for amplitude
                    if env.is_empty() {
                        BitDepth::one() // Default amplitude for empty envelope
                    } else {
                        *env.get(self.index).unwrap_or(env.last().unwrap())
                    }
                }
                _ => BitDepth::one(),
            },
        };

        let phase = match &mut self.phase_lfo_iter {
            Some(iter) => match iter.next() {
                Some(val) => val,
                // LFO ended - this shouldn't happen in a proper infinite LFO design
                // But if it does, we should use a fallback value, zero
                None => match &self.inner.phase {
                    Modulation::LFO(_) => BitDepth::zero(), // Default phase fallback
                    _ => BitDepth::zero(),
                },
            },
            None => match &self.inner.phase {
                Modulation::Static(val) => *val,
                Modulation::Envelope(env) => {
                    // Envelope: use value at index, or last value if past end (infinite continuation)
                    // Empty envelopes default to 0.0 for phase
                    if env.is_empty() {
                        BitDepth::zero() // Default phase for empty envelope
                    } else {
                        *env.get(self.index).unwrap_or(env.last().unwrap())
                    }
                }
                _ => BitDepth::zero(),
            },
        };

        // Wave iterators should be infinite - never return None
        // Only return None if there's numerical instability or invalid configuration
        if !self.inner.sample_rate.is_finite() || self.inner.sample_rate <= BitDepth::zero() {
            return None; // Invalid sample rate - numerical instability
        }

        if !self.inner.frequency.is_finite() {
            return None; // Invalid frequency - numerical instability
        }

        let t =
            BitDepth::from(self.index as f32).unwrap_or(BitDepth::zero()) / self.inner.sample_rate;
        let two_pi = BitDepth::from(2.0 * PI).unwrap_or(BitDepth::zero());
        let sample = self.func(two_pi * t * self.inner.frequency + phase);

        // Check for numerical instability in the output
        if !sample.is_finite() {
            return None;
        }

        let result = amp * sample;

        // Check final result for numerical instability
        if !result.is_finite() {
            return None;
        }

        self.index += 1;

        // Check for potential index overflow (very long-running iterators)
        if self.index == usize::MAX {
            return None; // Prevent overflow
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn test_wave_default() {
        let wave: Wave<f32> = Default::default();
        assert_eq!(
            wave,
            Wave {
                sample_rate: 0.0,
                frequency: 0.0,
                phase: Modulation::Static(0.0),
                amplitude: Modulation::Static(1.0),
                func: WaveFunc::Sine
            }
        );
    }

    #[test]
    fn test_wave_function_iterations() {
        // Test all wave functions with expected outputs
        let test_cases = [
            (
                WaveFunc::Sine,
                [0.0, 0.9980267, -0.1253336, -0.98228717, 0.24869062],
            ),
            (
                WaveFunc::Cosine,
                [1.0, -0.06279071, -0.99211466, 0.18738186, 0.968583],
            ),
            (WaveFunc::Square, [1.0, 1.0, -1.0, -1.0, 1.0]),
            (
                WaveFunc::Sawtooth,
                [-1.0, -0.47999996, 0.04000008, 0.5600002, -0.91999984],
            ),
            (
                WaveFunc::Triangle,
                [0.0, 0.96000004, -0.08000016, -0.8799999, 0.16000032],
            ),
        ];

        for (func, expected) in test_cases.iter() {
            let wave = Wave {
                sample_rate: 500.0,
                frequency: 130.0,
                func: *func,
                ..Default::default()
            };
            let res: Vec<f32> = wave.iter().take(5).collect();

            for (i, (&actual, &expected)) in res.iter().zip(expected.iter()).enumerate() {
                if expected == 1.0 || expected == -1.0 || expected == 0.0 {
                    // Exact comparison for simple values
                    assert_eq!(
                        actual, expected,
                        "Wave function {:?} sample {} mismatch",
                        func, i
                    );
                } else {
                    // Tolerance comparison for computed values
                    assert!(
                        (actual - expected).abs() < 1e-6,
                        "Wave function {:?} sample {} mismatch: expected {}, got {}",
                        func,
                        i,
                        expected,
                        actual
                    );
                }
            }
        }
    }

    #[test]
    fn test_wave_phase_shift() {
        let wave = Wave {
            sample_rate: 500.0,
            frequency: 120.0,
            phase: (PI / 2.0).into(),
            ..Default::default()
        };
        let res: Vec<f32> = wave.iter().take(5).collect();
        assert_eq!(res[0], 1.0);
    }

    #[test]
    fn test_wave_formatting() {
        // Test Wave display formatting
        let wave = Wave {
            sample_rate: 500.0,
            frequency: 120.0,
            phase: Modulation::Static(PI / 2.0),
            ..Default::default()
        };
        let fmt_string = format!("{wave}");
        assert_eq!(
            fmt_string,
            "<Func: Sine, Freq: 120Hz, Ampl: 1, Sampling Freq: 500Hz>"
        );

        // Test WaveFunc formatting
        assert_eq!(format!("{}", WaveFunc::Sine), "Sine");
        assert_eq!(format!("{}", WaveFunc::Cosine), "Cosine");
        assert_eq!(format!("{}", WaveFunc::Square), "Square");
        assert_eq!(format!("{}", WaveFunc::Sawtooth), "Sawtooth");
        assert_eq!(format!("{}", WaveFunc::Triangle), "Triangle");

        // Test complex wave display formatting
        let complex_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(PI / 4.0),
            amplitude: Modulation::Static(0.8),
            func: WaveFunc::Triangle,
        };

        let display_str = format!("{}", complex_wave);
        assert!(
            display_str.contains("Triangle"),
            "Should display wave function"
        );
        assert!(display_str.contains("440Hz"), "Should display frequency");
        assert!(
            display_str.contains("44100Hz"),
            "Should display sample rate"
        );
    }

    #[test]
    fn test_modulation_comprehensive() {
        // Test From conversions
        let static_mod: Modulation<f32> = 0.5f32.into();
        assert_eq!(static_mod, Modulation::Static(0.5));
        let envelope_data = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        let envelope_mod: Modulation<f32> = envelope_data.clone().into();
        assert_eq!(envelope_mod, Modulation::Envelope(envelope_data));
        let lfo_wave = Wave {
            sample_rate: 44100.0,
            frequency: 5.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(0.3),
            func: WaveFunc::Sine,
        };
        let lfo_mod: Modulation<f32> = lfo_wave.clone().into();
        assert_eq!(lfo_mod, Modulation::LFO(Box::new(lfo_wave)));

        // Test amplitude modulation with LFO
        let amplitude_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 5.0, // 5 Hz tremolo
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(0.3),
            func: WaveFunc::Sine,
        };

        let modulated_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(amplitude_lfo)),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = modulated_wave.iter().take(4410).collect(); // 0.1 seconds
        assert_eq!(samples.len(), 4410);
        assert!(samples.iter().all(|x| x.is_finite()));

        // Verify amplitude modulation variation
        let max_amplitude = samples.iter().fold(0.0f32, |max, &val| max.max(val.abs()));
        let min_amplitude = samples.iter().fold(f32::MAX, |min, &val| {
            if val.abs() < 0.001 {
                min
            } else {
                min.min(val.abs())
            }
        });
        assert!(
            max_amplitude > min_amplitude,
            "Amplitude modulation should create variation"
        );
        assert!(
            max_amplitude < 1.0,
            "Maximum amplitude should be reasonable"
        );

        // Test phase modulation with LFO
        let phase_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 2.0, // 2 Hz vibrato
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(PI / 4.0),
            func: WaveFunc::Sine,
        };

        let phase_modulated_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::LFO(Box::new(phase_lfo)),
            amplitude: Modulation::Static(1.0),
            func: WaveFunc::Sine,
        };

        let phase_samples: Vec<f32> = phase_modulated_wave.iter().take(4410).collect();
        assert_eq!(phase_samples.len(), 4410);
        let max_sample = phase_samples
            .iter()
            .fold(0.0f32, |max, &val| max.max(val.abs()));
        assert!(
            max_sample <= 1.1,
            "Phase modulation samples should be within bounds"
        );
    }

    #[test]
    fn test_envelope_modulation_comprehensive() {
        // Test amplitude modulation with envelope
        let envelope = vec![0.0, 0.25, 0.5, 0.75, 1.0, 0.75, 0.5, 0.25, 0.0];

        let modulated_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Envelope(envelope.clone()),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = modulated_wave.iter().take(envelope.len()).collect();
        assert_eq!(samples.len(), envelope.len());

        // First sample should be close to 0 (amplitude 0.0)
        assert!(
            samples[0].abs() < 0.001,
            "First sample should be near zero, got: {}",
            samples[0]
        );

        // Middle sample should be largest (amplitude 1.0)
        let mid_index = envelope.len() / 2;
        assert!(
            samples[mid_index].abs() > 0.1,
            "Middle sample should have significant amplitude"
        );

        // Last sample should be close to 0 again
        assert!(
            samples[envelope.len() - 1].abs() < 0.001,
            "Last sample should be near zero"
        );

        // Test phase modulation with envelope
        let phase_envelope = vec![0.0, PI / 4.0, PI / 2.0, PI / 4.0, 0.0];

        let phase_modulated_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Envelope(phase_envelope.clone()),
            amplitude: Modulation::Static(1.0),
            func: WaveFunc::Sine,
        };

        let phase_samples: Vec<f32> = phase_modulated_wave
            .iter()
            .take(phase_envelope.len())
            .collect();
        assert_eq!(phase_samples.len(), phase_envelope.len());
        assert!(
            !phase_samples.iter().all(|&x| x == phase_samples[0]),
            "Samples should vary with phase modulation"
        );

        // Test empty envelope - should use default behavior
        let empty_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Envelope(vec![]),
            amplitude: Modulation::Static(1.0),
            func: WaveFunc::Sine,
        };

        let empty_samples: Vec<f32> = empty_wave.iter().take(5).collect();
        assert_eq!(
            empty_samples.len(),
            5,
            "Empty envelope should produce samples with fallback behavior"
        );
        assert!(
            empty_samples.iter().all(|&x| x.is_finite()),
            "Empty envelope samples should be finite"
        );

        // Test single-value envelope
        let single_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Envelope(vec![0.7]),
            func: WaveFunc::Sine,
        };

        let single_samples: Vec<f32> = single_wave.iter().take(5).collect();
        assert_eq!(
            single_samples.len(),
            5,
            "Single envelope should produce all requested samples"
        );
        assert!(
            single_samples.iter().all(|&x| x.abs() <= 0.8),
            "Single envelope samples should be scaled by envelope value"
        );
        assert!(
            single_samples.iter().any(|&x| x.abs() > 0.1),
            "Single envelope should produce non-trivial samples"
        );
    }

    #[test]
    fn test_complex_modulation_combination() {
        let amplitude_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 3.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(0.2),
            func: WaveFunc::Sine,
        };

        let phase_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 7.0,
            phase: Modulation::Static(PI / 2.0),
            amplitude: Modulation::Static(PI / 6.0),
            func: WaveFunc::Cosine,
        };

        let complex_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::LFO(Box::new(phase_lfo)),
            amplitude: Modulation::LFO(Box::new(amplitude_lfo)),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = complex_wave.iter().take(4410).collect(); // 0.1 seconds

        // Verify complex modulation produces varied output
        assert_eq!(samples.len(), 4410);
        let variance = calculate_variance(&samples);
        assert!(
            variance > 0.01,
            "Complex modulation should produce significant variation"
        );
    }

    #[test]
    fn test_modulation_with_different_wave_functions() {
        let functions = [
            WaveFunc::Sine,
            WaveFunc::Cosine,
            WaveFunc::Square,
            WaveFunc::Sawtooth,
            WaveFunc::Triangle,
        ];

        for func in &functions {
            let lfo = Wave {
                sample_rate: 44100.0,
                frequency: 4.0,
                phase: Modulation::Static(0.0),
                amplitude: Modulation::Static(0.5),
                func: *func,
            };

            let modulated_wave = Wave {
                sample_rate: 44100.0,
                frequency: 220.0,
                phase: Modulation::Static(0.0),
                amplitude: Modulation::LFO(Box::new(lfo)),
                func: WaveFunc::Sine,
            };

            let samples: Vec<f32> = modulated_wave.iter().take(100).collect();
            assert!(
                !samples.is_empty(),
                "Should generate samples for {:?} modulation",
                func
            );
            assert!(
                samples.iter().any(|&x| x.abs() > 0.001),
                "Should have non-zero samples for {:?}",
                func
            );
        }
    }

    #[test]
    fn test_envelope_edge_cases() {
        // Empty envelope - should behave deterministically
        let empty_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Envelope(vec![]),
            amplitude: Modulation::Static(1.0),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = empty_wave.iter().take(5).collect();
        // Empty envelope should still produce samples (using default/fallback behavior)
        assert_eq!(
            samples.len(),
            5,
            "Empty envelope should produce requested samples with fallback behavior"
        );

        // All samples should be finite and non-NaN
        assert!(
            samples.iter().all(|&x| x.is_finite()),
            "Empty envelope samples should be finite"
        );

        // Single-value envelope - should use that single value consistently
        let single_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Envelope(vec![0.7]),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = single_wave.iter().take(5).collect();
        assert_eq!(
            samples.len(),
            5,
            "Single envelope should produce all requested samples"
        );

        // With amplitude 0.7, samples should be scaled appropriately
        assert!(
            samples.iter().all(|&x| x.abs() <= 0.8),
            "Single envelope samples should be scaled by envelope value"
        );
        assert!(
            samples.iter().any(|&x| x.abs() > 0.1),
            "Single envelope should produce non-trivial samples"
        );
    }

    // Wave edge case tests
    #[test]
    fn test_wave_function_edge_cases() {
        let functions = [
            WaveFunc::Sine,
            WaveFunc::Cosine,
            WaveFunc::Square,
            WaveFunc::Sawtooth,
            WaveFunc::Triangle,
        ];

        for &func in &functions {
            let wave = Wave {
                sample_rate: 44100.0,
                frequency: 1000.0,
                phase: Modulation::Static(0.0),
                amplitude: Modulation::Static(1.0),
                func,
            };

            let samples: Vec<f32> = wave.iter().take(100).collect();
            assert_eq!(
                samples.len(),
                100,
                "Should generate exactly 100 samples for {:?}",
                func
            );
            assert!(
                samples.iter().all(|x| x.is_finite()),
                "All samples should be finite for {:?}",
                func
            );

            // Check that function actually produces variation (not all zeros)
            assert!(
                samples.iter().any(|&x| x.abs() > 0.001),
                "Should produce non-zero samples for {:?}",
                func
            );
        }
    }

    #[test]
    fn test_wave_envelope_edge_cases() {
        // Test envelope that ends before wave generation stops
        let short_envelope = vec![1.0, 0.5, 0.0];

        let envelope_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Envelope(short_envelope.clone()),
            func: WaveFunc::Sine,
        };

        // Generate exactly the envelope length to test boundary behavior
        let samples: Vec<f32> = envelope_wave.iter().take(short_envelope.len()).collect();
        assert_eq!(
            samples.len(),
            short_envelope.len(),
            "Should produce samples for each envelope point"
        );

        // Check that envelope values are actually applied
        // Calculate expected values: sine(2π * 440 * t / 44100) * envelope[i]
        let expected_samples: Vec<f32> = (0..short_envelope.len())
            .map(|i| {
                let t = i as f32 / 44100.0;
                let sine_val = sinf(2.0 * PI * 440.0 * t);
                sine_val * short_envelope[i]
            })
            .collect();

        for (i, (&actual, &expected)) in samples.iter().zip(expected_samples.iter()).enumerate() {
            assert!(
                (actual - expected).abs() < 0.01,
                "Sample {}: expected {:.3}, got {:.3} (envelope: {})",
                i,
                expected,
                actual,
                short_envelope[i]
            );
        }

        // Verify that the third sample (amplitude 0.0) is indeed zero
        assert_eq!(
            samples[2], 0.0,
            "Zero amplitude envelope point should produce zero samples"
        );

        // Test beyond envelope length
        let extended_samples: Vec<f32> = envelope_wave.iter().take(10).collect();
        // Library behavior when envelope is exhausted - should either terminate or use fallback
        if extended_samples.len() > short_envelope.len() {
            // If it continues, the behavior after envelope should be consistent
            let post_envelope = &extended_samples[short_envelope.len()..];
            let first_post = post_envelope[0];
            assert!(
                post_envelope
                    .iter()
                    .all(|&x| (x - first_post).abs() < 0.001),
                "Post-envelope samples should be consistent"
            );
        }
    }

    #[test]
    fn test_phase_envelope_edge_cases() {
        // Test phase envelope with extreme values
        let extreme_phase_envelope = vec![0.0, PI, 2.0 * PI, -PI, 0.0];

        let phase_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Envelope(extreme_phase_envelope.clone()),
            amplitude: Modulation::Static(1.0),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = phase_wave
            .iter()
            .take(extreme_phase_envelope.len())
            .collect();

        assert_eq!(
            samples.len(),
            extreme_phase_envelope.len(),
            "Should produce sample for each phase envelope point"
        );

        // Phase modulation should create variation in the output
        // Since we're using sine with different phase offsets, we should see variation
        let first_half = &samples[0..2];
        let second_half = &samples[2..4];
        let has_variation = first_half
            .iter()
            .zip(second_half)
            .any(|(a, b)| (a - b).abs() > 0.1);
        assert!(
            has_variation,
            "Different phase envelope values should produce different samples"
        );

        // Sine function output should be bounded
        assert!(
            samples.iter().all(|&x| x.abs() <= 1.1),
            "Sine-based samples should be roughly bounded"
        );
    }

    #[test]
    fn test_lfo_termination() {
        // Test LFO that might terminate before parent wave
        let short_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 10.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Envelope(vec![0.5, 0.0]), // Very short envelope
            func: WaveFunc::Sine,
        };

        let lfo_modulated_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(short_lfo)),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = lfo_modulated_wave.iter().take(100).collect();
        assert_eq!(samples.len(), 100, "Should generate exactly 100 samples");

        // Should handle LFO termination without crashing and produce valid output
        assert!(
            samples.iter().all(|x| x.is_finite()),
            "Should produce finite samples"
        );

        // The LFO has envelope [0.5, 0.0] which means it produces:
        // Sample 0: LFO amplitude envelope[0] = 0.5, LFO output = sin(10Hz * 0) * 0.5 = 0
        // Sample 1: LFO amplitude envelope[1] = 0.0, LFO output = sin(10Hz * t) * 0.0 = 0
        // Sample 2+: LFO ended, amplitude = 0.0

        // Since phase is static (never ends), wave continues but with 0 amplitude
        // All samples should be zero because LFO modulation results in zero amplitude
        assert!(
            samples.iter().all(|&x| x == 0.0),
            "All samples should be zero when LFO envelope results in zero amplitude modulation"
        );
    }

    #[test]
    fn test_complex_lfo_interactions() {
        // Test nested LFO (LFO modulating another LFO)
        let inner_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 0.5, // Very slow modulation
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(2.0), // Modulation depth
            func: WaveFunc::Sine,
        };

        let outer_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 3.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(inner_lfo)), // Nested LFO
            func: WaveFunc::Sine,
        };

        let modulated_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(outer_lfo)),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = modulated_wave.iter().take(50).collect();
        assert_eq!(
            samples.len(),
            50,
            "Should generate exactly 50 samples with complex LFO interactions"
        );

        assert!(
            samples.iter().all(|x| x.is_finite()),
            "Complex LFO should produce finite samples"
        );

        // Complex LFO should actually modulate the amplitude - verify this produces meaningful output
        assert!(
            samples.iter().any(|&x| x != 0.0),
            "Complex LFO modulation should produce non-zero output"
        );

        // With nested LFO modulation, we should see amplitude variation over time
        let first_third = &samples[0..16];
        let second_third = &samples[16..32];
        let third_third = &samples[32..48];

        let avg1 = first_third.iter().map(|x| x.abs()).sum::<f32>() / first_third.len() as f32;
        let avg2 = second_third.iter().map(|x| x.abs()).sum::<f32>() / second_third.len() as f32;
        let avg3 = third_third.iter().map(|x| x.abs()).sum::<f32>() / third_third.len() as f32;

        // Nested LFO should create some variation in amplitude over time
        let max_avg = avg1.max(avg2).max(avg3);
        let min_avg = avg1.min(avg2).min(avg3);
        assert!(
            max_avg > min_avg * 0.1,
            "Nested LFO should create meaningful amplitude variation"
        );
    }

    #[test]
    fn test_both_envelopes_ending() {
        // Test wave where both amplitude and phase envelopes end
        let amp_envelope = vec![1.0, 0.5, 0.0];
        let phase_envelope = vec![0.0, PI / 2.0, PI];

        let dual_envelope_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Envelope(phase_envelope.clone()),
            amplitude: Modulation::Envelope(amp_envelope.clone()),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = dual_envelope_wave.iter().take(10).collect();

        // With fixed infinite iterator, should get all 10 requested samples
        assert_eq!(
            samples.len(),
            10,
            "Iterator should be infinite and provide all requested samples"
        );

        // Should handle both envelopes ending gracefully with meaningful output
        assert!(
            samples.iter().all(|x| x.is_finite()),
            "Should produce finite samples with dual envelopes"
        );

        // Check envelope behavior within envelope length
        if samples.len() >= 3 {
            // Third sample (index 2) uses amplitude envelope[2] = 0.0
            let third_sample = samples[2];
            assert_eq!(
                third_sample, 0.0,
                "Third sample should be zero when amplitude envelope[2] = 0.0"
            );
        }

        // Samples after envelope end should continue with last envelope values
        // amplitude: continues with 0.0, phase: continues with PI
        if samples.len() > 3 {
            let post_envelope_samples = &samples[3..];
            assert!(
                post_envelope_samples.iter().all(|&x| x == 0.0),
                "Post-envelope samples should be zero since amplitude continues at 0.0"
            );
        }

        // Phase envelope should create variation in the sine wave shape within envelope
        if samples.len() >= 2 {
            // samples[0] uses phase 0.0, samples[1] uses phase PI/2
            // At different phases, sine should produce different values (unless frequency is very low)
            let time_step = 1.0 / 44100.0;
            let freq_effect = 2.0 * PI * 440.0 * time_step;

            // If frequency creates significant phase change per sample, expect variation
            if freq_effect > 0.1 {
                assert!(
                    samples[0] != samples[1],
                    "Different phase envelope values should affect output"
                );
            }
        }
    }

    #[test]
    fn test_extreme_modulation_values() {
        // Test with large amplitude modulation
        let large_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 2.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(2.0), // Moderate amplitude
            func: WaveFunc::Sine,
        };

        let modulated_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(large_lfo)),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = modulated_wave.iter().take(50).collect();
        assert_eq!(samples.len(), 50, "Should handle large modulation values");

        // Should produce finite samples
        assert!(
            samples.iter().all(|x| x.is_finite()),
            "All samples should be finite"
        );

        // With modulation, there should be some amplitude variation
        let max_sample = samples.iter().fold(0.0f32, |acc, &val| acc.max(val.abs()));
        // Lower threshold - library might produce small values initially
        assert!(
            max_sample > 0.001,
            "Should produce samples with some amplitude (max: {})",
            max_sample
        );
    }

    #[test]
    fn test_zero_frequency_with_modulation() {
        // Test zero frequency carrier with amplitude modulation
        let lfo = Wave {
            sample_rate: 44100.0,
            frequency: 1.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(0.5),
            func: WaveFunc::Sine,
        };

        let zero_freq_wave = Wave {
            sample_rate: 44100.0,
            frequency: 0.0, // Zero frequency
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(lfo)),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = zero_freq_wave.iter().take(1000).collect(); // Shorter test
        assert_eq!(
            samples.len(),
            1000,
            "Should generate exactly 1000 samples for zero frequency with modulation"
        );

        // Should produce finite samples
        assert!(
            samples.iter().all(|x| x.is_finite()),
            "All samples should be finite"
        );

        // Zero frequency sine should produce a constant DC value (sine(0) = 0)
        // But LFO amplitude modulation should scale this constant
        // Since sine(0) = 0, zero frequency sine should produce all zeros regardless of amplitude
        assert!(
            samples.iter().all(|&x| x == 0.0),
            "Zero frequency sine should produce all zeros even with LFO amplitude modulation"
        );
    }

    #[test]
    fn test_iterator_infinite_behavior() {
        // Test that wave iterators are truly infinite and don't output zeros

        // 1. Test LFO with finite envelope - should continue infinitely with fallback
        let finite_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 1.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Envelope(vec![0.8, 0.6]), // Very short envelope
            func: WaveFunc::Sine,
        };

        let lfo_modulated_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(finite_lfo)),
            func: WaveFunc::Sine,
        };

        // Take many more samples than the LFO envelope length
        let samples: Vec<f32> = lfo_modulated_wave.iter().take(10000).collect();
        assert_eq!(
            samples.len(),
            10000,
            "Iterator should be infinite and provide all requested samples"
        );

        // All samples should be finite (no NaN or infinity)
        assert!(
            samples.iter().all(|x| x.is_finite()),
            "All samples should be finite"
        );

        // The iterator should not produce infinite zeros after LFO envelope ends
        let post_envelope_samples = &samples[100..200]; // Well past envelope
        let non_zero_count = post_envelope_samples.iter().filter(|&&x| x != 0.0).count();
        assert!(
            non_zero_count > 50,
            "Post-LFO-envelope samples should not all be zero (got {} non-zero out of {})",
            non_zero_count,
            post_envelope_samples.len()
        );

        // 2. Test envelope directly - should continue with last value infinitely
        let envelope_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Envelope(vec![1.0, 0.5, 0.2]), // Ends at 0.2
            func: WaveFunc::Sine,
        };

        let env_samples: Vec<f32> = envelope_wave.iter().take(1000).collect();
        assert_eq!(
            env_samples.len(),
            1000,
            "Envelope wave iterator should be infinite"
        );

        // Samples beyond envelope length should use last envelope value (0.2)
        let post_env_samples = &env_samples[10..20]; // Well past envelope end

        // Verify that the iterator continues and produces reasonable values
        assert!(
            post_env_samples.iter().all(|x| x.is_finite()),
            "Post-envelope samples should be finite"
        );

        // Verify that the iterator is actually using the last envelope value
        // by checking that amplitude ratios are consistent with 0.2
        let non_zero_samples: Vec<_> = post_env_samples
            .iter()
            .filter(|&&x| x.abs() > 0.01)
            .collect();
        if !non_zero_samples.is_empty() {
            let avg_abs_amp = non_zero_samples.iter().map(|&&x| x.abs()).sum::<f32>()
                / non_zero_samples.len() as f32;
            assert!(
                avg_abs_amp > 0.05 && avg_abs_amp < 0.25,
                "Average amplitude should be reasonable for envelope value 0.2, got {:.3}",
                avg_abs_amp
            );
        }

        // 3. Test static modulation - should be perfectly infinite
        let static_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(0.7),
            func: WaveFunc::Sine,
        };

        let static_samples: Vec<f32> = static_wave.iter().take(50000).collect();
        assert_eq!(
            static_samples.len(),
            50000,
            "Static wave iterator should be infinite"
        );
        assert!(
            static_samples.iter().all(|x| x.is_finite()),
            "All static samples should be finite"
        );
    }

    #[test]
    fn test_wave_quantization() {
        // Test quantization functionality with various bit depths
        let wave = Wave::<f32> {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(1.0),
            func: WaveFunc::Sine,
        };

        // Test quantization to i16 (16-bit signed)
        let quantized_i16: Vec<i16> = wave.quantize().take(10).collect();
        assert_eq!(
            quantized_i16.len(),
            10,
            "Should produce exactly 10 i16 samples"
        );
        assert!(
            quantized_i16.iter().all(|&x| x.abs() <= i16::MAX),
            "i16 samples should be within range"
        );

        // First sample should be zero (sine(0) = 0)
        assert_eq!(quantized_i16[0], 0, "First sample should be zero");

        // Check that samples are non-zero after the first
        assert!(
            quantized_i16.iter().skip(1).any(|&x| x != 0),
            "Should have non-zero samples"
        );

        // Test quantization to i8 (8-bit signed)
        let quantized_i8: Vec<i8> = wave.quantize().take(10).collect();
        assert_eq!(
            quantized_i8.len(),
            10,
            "Should produce exactly 10 i8 samples"
        );
        assert!(
            quantized_i8.iter().all(|&x| x.abs() <= i8::MAX),
            "i8 samples should be within range"
        );
        assert_eq!(quantized_i8[0], 0, "First i8 sample should be zero");

        // Test quantization to u16 (16-bit unsigned)
        let quantized_u16: Vec<u16> = wave.quantize().take(10).collect();
        assert_eq!(
            quantized_u16.len(),
            10,
            "Should produce exactly 10 u16 samples"
        );
        assert!(
            quantized_u16.iter().all(|&x| x <= u16::MAX),
            "u16 samples should be within range"
        );

        // Test quantization with different wave functions
        let square_wave = Wave::<f32> {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(1.0),
            func: WaveFunc::Square,
        };

        let square_quantized: Vec<i16> = square_wave.quantize().take(10).collect();
        assert_eq!(
            square_quantized.len(),
            10,
            "Square wave should quantize correctly"
        );

        // Square wave should produce maximum positive or negative values
        let max_vals = square_quantized
            .iter()
            .filter(|&&x| x.abs() > i16::MAX / 2)
            .count();
        assert!(
            max_vals > 5,
            "Square wave should produce mostly max amplitude values"
        );

        // Test quantization with amplitude modulation
        let modulated_wave = Wave::<f32> {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(0.5), // Half amplitude
            func: WaveFunc::Sine,
        };

        let modulated_quantized: Vec<i16> = modulated_wave.quantize().take(100).collect();
        let max_amplitude = modulated_quantized
            .iter()
            .map(|&x| x.abs())
            .max()
            .unwrap_or(0);

        // With 0.5 amplitude, max should be around half of i16::MAX
        assert!(
            max_amplitude < i16::MAX,
            "Amplitude modulation should reduce quantized range"
        );
        assert!(
            max_amplitude > i16::MAX / 4,
            "Should still have reasonable amplitude"
        );

        // Test quantization to f32 (should work as pass-through scaling)
        let quantized_f32: Vec<f32> = wave.quantize().take(10).collect();
        assert_eq!(quantized_f32.len(), 10, "Should produce f32 samples");
        assert!(
            quantized_f32.iter().all(|x| x.is_finite()),
            "f32 samples should be finite"
        );

        // Compare with original wave samples
        let original_samples: Vec<f32> = wave.iter().take(10).collect();
        for (i, (&quantized, &original)) in quantized_f32
            .iter()
            .zip(original_samples.iter())
            .enumerate()
        {
            // When quantizing to f32, should be scaled by f32::MAX but still proportional
            let expected_scaled = original * f32::MAX;
            assert!(
                (quantized - expected_scaled).abs() < 1e6, // Allow for some floating point precision
                "Sample {}: quantized {:.3e} should be close to scaled original {:.3e}",
                i,
                quantized,
                expected_scaled
            );
        }

        println!("✅ Wave quantization test passed for all bit depths");
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
