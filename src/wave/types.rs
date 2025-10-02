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

//! Core wave types and enums.

use alloc::{boxed::Box, string::ToString, vec::Vec};
use core::fmt;
use num_traits::Float;

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
    pub fn iter(&self) -> super::iterator::WaveIterator<'_, BitDepth> {
        self.into_iter()
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
