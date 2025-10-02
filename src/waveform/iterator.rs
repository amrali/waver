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

//! Waveform iterator implementation.

use super::types::{Waveform, WaveformSource};
use crate::WaveIterator;
use alloc::vec::Vec;
use core::iter::IntoIterator;
use num_traits::Float;

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
    pub(crate) _inner: &'a Waveform<BitDepth>,
    pub(crate) source: WaveformIteratorSource<'a, BitDepth>,
}

impl<'a, BitDepth: Float + Copy> WaveformIterator<'a, BitDepth> {
    /// Create a new WaveformIterator.
    pub(crate) fn new(
        _inner: &'a Waveform<BitDepth>,
        source: WaveformIteratorSource<'a, BitDepth>,
    ) -> Self {
        Self { _inner, source }
    }
}

impl<'a, BitDepth: Float + Copy + core::iter::Sum> IntoIterator for &'a Waveform<BitDepth> {
    type Item = BitDepth;
    type IntoIter = WaveformIterator<'a, BitDepth>;

    fn into_iter(self) -> Self::IntoIter {
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
