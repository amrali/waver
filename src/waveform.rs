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
    pub fn iter(&self) -> WaveformIterator<BitDepth> {
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
                if iters.is_empty() {
                    return Some(NumCast::from(0.0).unwrap());
                }
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
    use crate::{Modulation, Wave};
    use core::i16;

    #[test]
    fn test_waveform_single_wave_match() {
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
    fn test_waveform_amplitude_normalization() {
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

        if let WaveformSource::Generative(components) = wf.source {
            components
                .iter()
                .for_each(|c| assert_eq!(c.amplitude, Modulation::Static(0.5)));
        } else {
            panic!("Expected generative waveform");
        }
    }

    #[test]
    fn test_superpose_on_recorded_error() {
        let mut wf = Waveform::<i16>::from_recorded_samples(44100.0, &[]);
        let err = wf.superpose(Wave::default()).unwrap_err();
        assert_eq!(err, Error::UnsupportedSource);
    }

    #[test]
    fn test_normalize_on_recorded_error() {
        let mut wf = Waveform::<i16>::from_recorded_samples(44100.0, &[]);
        let err = wf.normalize_amplitudes().unwrap_err();
        assert_eq!(err, Error::UnsupportedSource);
    }

    #[test]
    fn test_waveform_iterator_halt_numerical_instability() {
        let mut wf = Waveform::<i16>::with_wave(
            44100.0,
            Wave {
                frequency: 4000.0,
                amplitude: 1.0.into(),
                ..Default::default()
            },
        );
        wf.superpose(Wave {
            frequency: 5000.0,
            amplitude: 0.5.into(),
            ..Default::default()
        })
        .unwrap();
        let v: Vec<i16> = wf.iter().take(100).collect();

        assert_ne!(v.len(), 100);
    }

    #[test]
    fn test_from_recorded_samples() {
        let samples = vec![0, 1, 2, 3, 4];
        let wf = Waveform::<i16>::from_recorded_samples(44100.0, &samples);
        if let WaveformSource::Recorded(data) = wf.source {
            assert_eq!(data, samples);
        } else {
            panic!("Expected recorded waveform");
        }
    }

    #[test]
    fn test_recorded_waveform_iteration() {
        let samples = vec![0, 1, 2, 3, 4];
        let wf = Waveform::<i16>::from_recorded_samples(44100.0, &samples);
        let collected: Vec<i16> = wf.iter().collect();
        assert_eq!(collected, samples);
    }
}
