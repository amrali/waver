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

//! Wave iterator implementation.

use super::types::{Modulation, Wave, WaveFunc};
use alloc::boxed::Box;
use core::{f32::consts::PI, iter::IntoIterator};
use libm::{asinf, copysignf, cosf, sinf};
use num_traits::Float;

/// Iterator for Wave structure.
pub struct WaveIterator<'a, BitDepth: Float + Copy> {
    inner: &'a Wave<BitDepth>,
    index: usize,
    amp_lfo_iter: Option<Box<WaveIterator<'a, BitDepth>>>,
    phase_lfo_iter: Option<Box<WaveIterator<'a, BitDepth>>>,
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
