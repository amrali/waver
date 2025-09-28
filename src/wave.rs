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
pub enum Modulation {
    /// A static, constant value.
    Static(f32),
    /// A pre-calculated envelope.
    Envelope(Vec<f32>),
    /// A Low-Frequency Oscillator (LFO) that modulates the property.
    LFO(Box<Wave>),
}

impl From<f32> for Modulation {
    fn from(val: f32) -> Self {
        Modulation::Static(val)
    }
}

impl From<Vec<f32>> for Modulation {
    fn from(val: Vec<f32>) -> Self {
        Modulation::Envelope(val)
    }
}

impl From<Wave> for Modulation {
    fn from(val: Wave) -> Self {
        Modulation::LFO(Box::new(val))
    }
}

/// A structure that represent a sinusoidal wave, with potentially dynamic
/// amplitude and phase.
///
/// The default value for a wave values is 0.0 except for the amplitude weight
/// which is 1.0 (100% of available amplitude).
#[derive(Debug, Clone, PartialEq)]
pub struct Wave {
    /// The sampling rate of this wave.
    pub sample_rate: f32,
    /// The frequency of this wave.
    pub frequency: f32,
    /// The phase of this wave.
    pub phase: Modulation,
    /// The amplitude as a percentage [0.0 - 1.0].
    pub amplitude: Modulation,
    /// The trigonometric function to express the wave.
    pub func: WaveFunc,
}

impl Wave {
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
    pub fn iter(&self) -> WaveIterator<'_> {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a Wave {
    type Item = f32;
    type IntoIter = WaveIterator<'a>;

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

impl Default for Wave {
    fn default() -> Self {
        Self {
            sample_rate: 0.0,
            frequency: 0.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(1.0),
            func: WaveFunc::Sine,
        }
    }
}

impl fmt::Display for Wave {
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
pub struct WaveIterator<'a> {
    inner: &'a Wave,
    index: usize,
    amp_lfo_iter: Option<Box<WaveIterator<'a>>>,
    phase_lfo_iter: Option<Box<WaveIterator<'a>>>,
}

impl<'a> WaveIterator<'a> {
    /// Resolve the wave function.
    #[inline]
    fn func(&self, x: f32) -> f32 {
        match self.inner.func {
            WaveFunc::Sine => sinf(x),
            WaveFunc::Cosine => cosf(x),
            WaveFunc::Square => copysignf(1.0, sinf(x)),
            WaveFunc::Sawtooth => (x % (2.0 * PI)) / PI - 1.0,
            WaveFunc::Triangle => 2.0 * asinf(sinf(x)) / PI,
        }
    }
}

impl<'a> Iterator for WaveIterator<'a> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let amp = match &mut self.amp_lfo_iter {
            Some(iter) => match iter.next() {
                Some(val) => val,
                // LFO ended - this shouldn't happen in a proper infinite LFO design
                // But if it does, we should use a fallback value, not zero
                None => match &self.inner.amplitude {
                    Modulation::LFO(_) => 1.0, // Default amplitude fallback
                    _ => 1.0,
                },
            },
            None => match &self.inner.amplitude {
                Modulation::Static(val) => *val,
                Modulation::Envelope(env) => {
                    // Envelope: use value at index, or last value if past end (infinite continuation)
                    // Empty envelopes default to 1.0 for amplitude
                    if env.is_empty() {
                        1.0 // Default amplitude for empty envelope
                    } else {
                        *env.get(self.index).unwrap_or(env.last().unwrap())
                    }
                }
                _ => 1.0,
            },
        };

        let phase = match &mut self.phase_lfo_iter {
            Some(iter) => match iter.next() {
                Some(val) => val,
                // LFO ended - this shouldn't happen in a proper infinite LFO design
                // But if it does, we should use a fallback value, not zero
                None => match &self.inner.phase {
                    Modulation::LFO(_) => 0.0, // Default phase fallback
                    _ => 0.0,
                },
            },
            None => match &self.inner.phase {
                Modulation::Static(val) => *val,
                Modulation::Envelope(env) => {
                    // Envelope: use value at index, or last value if past end (infinite continuation)
                    // Empty envelopes default to 0.0 for phase
                    if env.is_empty() {
                        0.0 // Default phase for empty envelope
                    } else {
                        *env.get(self.index).unwrap_or(env.last().unwrap())
                    }
                }
                _ => 0.0,
            },
        };

        // Wave iterators should be infinite - never return None
        // Only return None if there's numerical instability or invalid configuration
        if !self.inner.sample_rate.is_finite() || self.inner.sample_rate <= 0.0 {
            return None; // Invalid sample rate - numerical instability
        }

        if !self.inner.frequency.is_finite() {
            return None; // Invalid frequency - numerical instability
        }

        let t = self.index as f32 / self.inner.sample_rate;
        let sample = self.func(2.0 * PI * t * self.inner.frequency + phase);

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
        let wave: Wave = Default::default();
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
        let static_mod: Modulation = 0.5f32.into();
        assert_eq!(static_mod, Modulation::Static(0.5));
        let envelope_data = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        let envelope_mod: Modulation = envelope_data.clone().into();
        assert_eq!(envelope_mod, Modulation::Envelope(envelope_data));
        let lfo_wave = Wave {
            sample_rate: 44100.0,
            frequency: 5.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(0.3),
            func: WaveFunc::Sine,
        };
        let lfo_mod: Modulation = lfo_wave.clone().into();
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
    fn test_complex_modulation_scenarios() {
        // Test basic complex modulation combination
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
        assert_eq!(samples.len(), 4410);
        let variance = calculate_variance(&samples);
        assert!(
            variance > 0.01,
            "Complex modulation should produce significant variation"
        );

        // Test modulation with different wave functions as LFOs
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

            let func_samples: Vec<f32> = modulated_wave.iter().take(100).collect();
            assert!(
                !func_samples.is_empty(),
                "Should generate samples for {:?} modulation",
                func
            );
            assert!(
                func_samples.iter().any(|&x| x.abs() > 0.001),
                "Should have non-zero samples for {:?}",
                func
            );
        }

        // Test nested LFO interactions
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

        let nested_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(outer_lfo)),
            func: WaveFunc::Sine,
        };

        let nested_samples: Vec<f32> = nested_wave.iter().take(50).collect();
        assert_eq!(
            nested_samples.len(),
            50,
            "Should generate samples with complex LFO interactions"
        );
        assert!(
            nested_samples.iter().all(|x| x.is_finite()),
            "Complex LFO should produce finite samples"
        );
        assert!(
            nested_samples.iter().any(|&x| x != 0.0),
            "Complex LFO modulation should produce non-zero output"
        );
    }

    #[test]
    fn test_envelope_and_iterator_edge_cases() {
        // Test envelope that ends before wave generation stops
        let short_envelope = vec![1.0, 0.5, 0.0];

        let envelope_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Envelope(short_envelope.clone()),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = envelope_wave.iter().take(short_envelope.len()).collect();
        assert_eq!(
            samples.len(),
            short_envelope.len(),
            "Should produce samples for each envelope point"
        );

        // Verify that the third sample (amplitude 0.0) is indeed zero
        assert_eq!(
            samples[2], 0.0,
            "Zero amplitude envelope point should produce zero samples"
        );

        // Test phase envelope with extreme values
        let extreme_phase_envelope = vec![0.0, PI, 2.0 * PI, -PI, 0.0];

        let phase_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Envelope(extreme_phase_envelope.clone()),
            amplitude: Modulation::Static(1.0),
            func: WaveFunc::Sine,
        };

        let phase_samples: Vec<f32> = phase_wave
            .iter()
            .take(extreme_phase_envelope.len())
            .collect();
        assert_eq!(
            phase_samples.len(),
            extreme_phase_envelope.len(),
            "Should produce sample for each phase envelope point"
        );

        // Phase modulation should create variation in the output
        let first_half = &phase_samples[0..2];
        let second_half = &phase_samples[2..4];
        let has_variation = first_half
            .iter()
            .zip(second_half)
            .any(|(a, b)| (a - b).abs() > 0.1);
        assert!(
            has_variation,
            "Different phase envelope values should produce different samples"
        );
        assert!(
            phase_samples.iter().all(|&x| x.abs() <= 1.1),
            "Sine-based samples should be roughly bounded"
        );

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

        let lfo_samples: Vec<f32> = lfo_modulated_wave.iter().take(100).collect();
        assert_eq!(
            lfo_samples.len(),
            100,
            "Should generate exactly 100 samples"
        );
        assert!(
            lfo_samples.iter().all(|x| x.is_finite()),
            "Should produce finite samples"
        );

        // LFO envelope results in zero amplitude modulation
        assert!(
            lfo_samples.iter().all(|&x| x == 0.0),
            "All samples should be zero when LFO envelope results in zero amplitude modulation"
        );
    }

    // Wave edge case tests
    #[test]
    fn test_wave_function_and_edge_cases() {
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
            assert!(
                samples.iter().any(|&x| x.abs() > 0.001),
                "Should produce non-zero samples for {:?}",
                func
            );
        }
    }

    #[test]
    fn test_comprehensive_iterator_behavior() {
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

        // With infinite iterator, should get all 10 requested samples
        assert_eq!(
            samples.len(),
            10,
            "Iterator should be infinite and provide all requested samples"
        );
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

        // Test extreme modulation values
        let large_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 2.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(2.0), // Moderate amplitude
            func: WaveFunc::Sine,
        };

        let extreme_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(large_lfo)),
            func: WaveFunc::Sine,
        };

        let extreme_samples: Vec<f32> = extreme_wave.iter().take(50).collect();
        assert_eq!(
            extreme_samples.len(),
            50,
            "Should handle large modulation values"
        );
        assert!(
            extreme_samples.iter().all(|x| x.is_finite()),
            "All samples should be finite"
        );

        // Test zero frequency with modulation
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

        let zero_samples: Vec<f32> = zero_freq_wave.iter().take(100).collect();
        assert_eq!(
            zero_samples.len(),
            100,
            "Should generate samples for zero frequency with modulation"
        );
        assert!(
            zero_samples.iter().all(|x| x.is_finite()),
            "All samples should be finite"
        );
        // Zero frequency sine should produce all zeros regardless of amplitude modulation
        assert!(
            zero_samples.iter().all(|&x| x == 0.0),
            "Zero frequency sine should produce all zeros"
        );

        // Test that iterators are truly infinite and maintain proper behavior
        let finite_lfo = Wave {
            sample_rate: 44100.0,
            frequency: 1.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Envelope(vec![0.8, 0.6]), // Very short envelope
            func: WaveFunc::Sine,
        };

        let infinite_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(finite_lfo)),
            func: WaveFunc::Sine,
        };

        let infinite_samples: Vec<f32> = infinite_wave.iter().take(1000).collect();
        assert_eq!(
            infinite_samples.len(),
            1000,
            "Iterator should be infinite and provide all requested samples"
        );
        assert!(
            infinite_samples.iter().all(|x| x.is_finite()),
            "All samples should be finite"
        );

        // Test envelope directly - should continue with last value infinitely
        let envelope_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Envelope(vec![1.0, 0.5, 0.2]), // Ends at 0.2
            func: WaveFunc::Sine,
        };

        let env_samples: Vec<f32> = envelope_wave.iter().take(100).collect();
        assert_eq!(
            env_samples.len(),
            100,
            "Envelope wave iterator should be infinite"
        );
        let post_env_samples = &env_samples[10..20]; // Well past envelope end
        assert!(
            post_env_samples.iter().all(|x| x.is_finite()),
            "Post-envelope samples should be finite"
        );

        // Test static modulation - should be perfectly infinite
        let static_wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(0.7),
            func: WaveFunc::Sine,
        };

        let static_samples: Vec<f32> = static_wave.iter().take(1000).collect();
        assert_eq!(
            static_samples.len(),
            1000,
            "Static wave iterator should be infinite"
        );
        assert!(
            static_samples.iter().all(|x| x.is_finite()),
            "All static samples should be finite"
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
