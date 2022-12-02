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

use crate::{Wave, WaveIterator};
use alloc::vec::Vec;
use core::{
    iter::{IntoIterator, Iterator},
    marker::PhantomData,
};
use num_traits::{AsPrimitive, Bounded, NumCast};

/// A structure that represent a waveform.
#[derive(Debug, Clone)]
pub struct Waveform<BitDepth: Clone> {
    sample_rate: f32,
    components: Vec<Wave>,
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
            components: Vec::new(),
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
        Self::new(sample_rate).superpose(wave).clone()
    }

    /// Add a wave component.
    ///
    /// # Examples
    ///
    /// ```
    /// use waver::{Waveform, Wave};
    ///
    /// let mut wf = Waveform::<i16>::new(44100.0);
    /// wf.superpose(Wave { frequency: 6000.0, amplitude: 0.25, ..Default::default() })
    ///     .superpose(Wave { frequency: 5500.0, amplitude: 0.75, ..Default::default() });
    /// ```
    pub fn superpose(&mut self, wave: Wave) -> &mut Self {
        self.components.push(Wave {
            sample_rate: self.sample_rate,
            ..wave
        });
        self
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
    /// # Examples
    ///
    /// ```
    /// use waver::{Waveform, Wave};
    ///
    /// // Two waves with an amplitude weights of 150%.
    /// let mut wf = Waveform::<i16>::with_wave(44100.0,
    ///     Wave { frequency: 3000.0, amplitude: 1.0, ..Default::default() });
    /// wf.superpose(Wave { frequency: 4000.0, amplitude: 0.5, ..Default::default() }).normalize_amplitudes();
    /// ```
    pub fn normalize_amplitudes(&mut self) -> &mut Self {
        let amp_ratio = 1.0 / self.components.len() as f32;
        self.components
            .iter_mut()
            .for_each(|c| c.amplitude = amp_ratio);
        self
    }

    /// An infinite iterator for the superposition of all underlying waveform components.
    ///
    /// The iterator will produce a quantization of the superposition of waveform
    /// components at the waveform sampling frequency.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::f32::consts::PI;
    /// use std::vec::Vec;
    /// use waver::{Waveform, Wave};
    ///
    /// let mut wf = Waveform::<i16>::new(44100.0);
    /// let res: Vec<i16> = wf.superpose(Wave { frequency: 4000.0, phase: PI / 2.0, ..Default::default() })
    ///     .iter().take(10).collect();
    /// ```
    pub fn iter(&self) -> WaveformIterator<BitDepth> {
        WaveformIterator {
            _inner: self,
            iters: self.components.iter().map(|c| c.iter()).collect(),
        }
    }
}

impl<'a, BitDepth: Bounded + NumCast + AsPrimitive<f32>> IntoIterator for &'a Waveform<BitDepth> {
    type Item = BitDepth;
    type IntoIter = WaveformIterator<'a, BitDepth>;

    fn into_iter(self) -> Self::IntoIter {
        WaveformIterator {
            _inner: self,
            iters: self.components.iter().map(|c| c.iter()).collect(),
        }
    }
}

/// Iterator for Waveform structure.
#[derive(Debug, Clone)]
pub struct WaveformIterator<'a, BitDepth: Clone> {
    _inner: &'a Waveform<BitDepth>,
    iters: Vec<WaveIterator<'a>>,
}

impl<'a, BitDepth: Bounded + NumCast + AsPrimitive<f32>> Iterator
    for WaveformIterator<'a, BitDepth>
{
    type Item = BitDepth;

    fn next(&mut self) -> Option<Self::Item> {
        // Superpose all waveform components.
        let superposition: f32 = self
            .iters
            .iter_mut()
            .map(|x| x.next().expect("waves are infinite"))
            .sum();
        NumCast::from(superposition * BitDepth::max_value().as_())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::i16;

    #[test]
    fn test_waveform_single_wave_match() {
        let w3khz = Wave {
            sample_rate: 44100.0,
            frequency: 3000.0,
            ..Default::default()
        };
        let wf = Waveform::<i16>::with_wave(44100.0, w3khz);

        let w1: Vec<i16> = wf.iter().take(100).collect();
        let w2: Vec<i16> = w3khz
            .iter()
            .take(100)
            .map(|c| (c * i16::MAX as f32) as i16)
            .collect();

        assert_eq!(w1, w2);
    }

    #[test]
    fn test_waveform_empty_zeros() {
        let wf = Waveform::<i8>::new(44100.0);
        let v: Vec<i8> = wf.iter().take(10).collect();
        assert_eq!(v, [0i8; 10]);
    }

    #[test]
    fn test_waveform_iteration() {
        let wf = Waveform::<i8>::new(44100.0);
        let mut itr = wf.into_iter();

        assert_eq!(itr.next().unwrap(), 0);
    }

    #[test]
    fn test_waveform_construct() {
        let wf1 = Waveform::<i16>::with_wave(
            44100.0,
            Wave {
                frequency: 3400.0,
                amplitude: 1.0,
                ..Default::default()
            },
        );
        let mut wf2 = Waveform::<i16>::new(44100.0);

        let v1: Vec<i16> = wf1.iter().take(100).collect();
        let v2: Vec<i16> = wf2
            .superpose(Wave {
                frequency: 3400.0,
                ..Default::default()
            })
            .iter()
            .take(100)
            .collect();

        assert_eq!(v1, v2);
    }

    #[test]
    fn test_waveform_amplitude_normalization() {
        let mut wf = Waveform::<i16>::with_wave(
            44100.0,
            Wave {
                frequency: 4000.0,
                amplitude: 1.5,
                ..Default::default()
            },
        );
        wf.superpose(Wave {
            frequency: 5000.0,
            amplitude: 0.5,
            ..Default::default()
        })
        .normalize_amplitudes();

        wf.components
            .iter()
            .for_each(|c| assert_eq!(c.amplitude, 0.5));
    }

    #[test]
    fn test_waveform_iterator_halt_numerical_instability() {
        let mut wf = Waveform::<i16>::with_wave(
            44100.0,
            Wave {
                frequency: 4000.0,
                amplitude: 1.0,
                ..Default::default()
            },
        );
        let v: Vec<i16> = wf
            .superpose(Wave {
                frequency: 5000.0,
                amplitude: 0.5,
                ..Default::default()
            })
            .iter()
            .take(100)
            .collect();

        assert_ne!(v.len(), 100);
    }
}
