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

use core::{
    f32::consts::PI,
    fmt,
    iter::{IntoIterator, Iterator},
};
use libm::sinf;

/// An enum that represents the kind of the wave function.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum WaveFunc {
    /// The sine function.
    Sine,

    /// The cosine function.
    Cosine,
}

impl fmt::Display for WaveFunc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Sine => "Sine",
                Self::Cosine => "Cosine",
            }
        )
    }
}

/// A structure that represent a sinusoidal wave.
///
/// The default value for a wave values is 0.0 except for the amplitude weight
/// which is 1.0 (100% of available amplitude).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wave {
    /// The sampling rate of this wave.
    pub sample_rate: f32,

    /// The frequency of this wave.
    pub frequency: f32,

    /// The phase of this wave.
    pub phase: f32,

    /// The amplitude as a percentage [0.0 - 1.0].
    pub amplitude: f32,

    /// The trignomic function to express the wave.
    pub wavefunc: WaveFunc,
}

impl Wave {
    /// An infinite iterator for the Wave structure.
    ///
    /// The iterator will produce an infinite number of wave samples at the
    /// specified sampling rate and frequency.
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
    pub fn iter(&self) -> WaveIterator {
        WaveIterator {
            inner: self,
            index: 0.0,
        }
    }
}

impl<'a> IntoIterator for &'a Wave {
    type Item = f32;
    type IntoIter = WaveIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        WaveIterator {
            inner: self,
            index: 0.0,
        }
    }
}

impl Default for Wave {
    fn default() -> Self {
        Self {
            sample_rate: 0.0,
            frequency: 0.0,
            phase: 0.0,
            amplitude: 1.0,
            wavefunc: WaveFunc::Sine,
        }
    }
}

impl fmt::Display for Wave {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<Func: {}, Freq: {}Hz, Ampl: {}, Sampling Freq: {}Hz>",
            self.wavefunc, self.frequency, self.amplitude, self.sample_rate
        )
    }
}

/// Iterator for Wave structure.
#[derive(Debug, Clone)]
pub struct WaveIterator<'a> {
    inner: &'a Wave,
    index: f32,
}

impl<'a> WaveIterator<'a> {
    /// Post-increment the index of the iterator.
    #[inline]
    fn index_inc(&mut self) -> f32 {
        let idx = self.index;
        // The index cycles after 1s of samples.
        self.index = (self.index % self.inner.sample_rate) + 1.0;

        idx
    }

    /// Resolve the wave function.
    #[inline]
    fn func(&self, x: f32) -> f32 {
        match self.inner.wavefunc {
            WaveFunc::Sine => sinf(x),
            WaveFunc::Cosine => sinf(PI / 2.0 - x),
        }
    }
}

impl<'a> Iterator for WaveIterator<'a> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let t = self.index_inc() / self.inner.sample_rate;

        Some(
            self.inner.amplitude
                * self.func(2.0 * PI * t * self.inner.frequency + self.inner.phase),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn test_wave_default() {
        let wave: Wave = Default::default();

        assert_eq!(
            wave,
            Wave {
                sample_rate: 0.0,
                frequency: 0.0,
                phase: 0.0,
                amplitude: 1.0,
                wavefunc: WaveFunc::Sine
            }
        );
    }

    #[test]
    fn test_wave_iteration() {
        let wave = Wave {
            sample_rate: 500.0,
            frequency: 130.0,
            ..Default::default()
        };
        let res: Vec<f32> = wave.iter().take(1001).collect();

        // It must start from the point of origin.
        assert_eq!(res[0], 0.0);

        // The 2s of samples must match exactly.
        assert_eq!(&res[1..501], &res[501..]);
    }

    #[test]
    fn test_wave_iteration_cosine() {
        let wave = Wave {
            sample_rate: 500.0,
            frequency: 130.0,
            wavefunc: WaveFunc::Cosine,
            ..Default::default()
        };
        let res: Vec<f32> = wave.iter().take(1001).collect();

        // It must start from the point of origin.
        assert_eq!(res[0], 1.0);

        // The 2s of samples must match exactly.
        assert_eq!(&res[1..501], &res[501..]);
    }

    #[test]
    fn test_wave_phase_shift() {
        let wave = Wave {
            sample_rate: 500.0,
            frequency: 120.0,
            phase: PI / 2.0,
            ..Default::default()
        };
        let res: Vec<f32> = wave.iter().take(5).collect();

        // A cosine wave is a sine wave with a phase shift of Pi / 2.
        assert_eq!(res[0], 1.0);
    }
}
