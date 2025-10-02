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

//! Waveform types and structures.

use crate::{error::Error, Wave};
use alloc::vec::Vec;
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
    pub fn iter(&self) -> super::iterator::WaveformIterator<'_, BitDepth> {
        use super::iterator::{WaveformIterator, WaveformIteratorSource};

        let iter_source = match &self.source {
            WaveformSource::Generative(components) => {
                let iters = components.iter().map(|c| c.iter()).collect();
                WaveformIteratorSource::Generative(iters)
            }
            WaveformSource::Recorded(data) => WaveformIteratorSource::Recorded(data.iter()),
        };

        WaveformIterator::new(self, iter_source)
    }
}
