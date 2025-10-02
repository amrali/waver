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

//! Iterator types for streaming quantization and dequantization.

use num_traits::{Bounded, Float, NumCast};

/// Dequantization iterator that lazily converts integer samples to floating-point.
///
/// This iterator allows for efficient streaming dequantization without allocating
/// intermediate vectors.
pub struct DequantizationIterator<I, Q, F>
where
    I: Iterator<Item = Q>,
    Q: Bounded + NumCast + Copy,
    F: Float + NumCast,
{
    inner: I,
    max_val_f: F,
    _phantom_q: core::marker::PhantomData<Q>,
}

impl<I, Q, F> DequantizationIterator<I, Q, F>
where
    I: Iterator<Item = Q>,
    Q: Bounded + NumCast + Copy,
    F: Float + NumCast,
{
    /// Create a new dequantization iterator.
    pub fn new(inner: I) -> Self {
        let max_val = Q::max_value();
        let max_val_f: F = NumCast::from(max_val).unwrap_or(F::one());
        Self {
            inner,
            max_val_f,
            _phantom_q: core::marker::PhantomData,
        }
    }
}

impl<I, Q, F> Iterator for DequantizationIterator<I, Q, F>
where
    I: Iterator<Item = Q>,
    Q: Bounded + NumCast + Copy,
    F: Float + NumCast,
{
    type Item = F;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        let sample_f: F = NumCast::from(sample).unwrap_or(F::zero());
        Some(sample_f / self.max_val_f)
    }
}

/// Quantization iterator that lazily converts samples.
///
/// This iterator allows for efficient streaming quantization without allocating
/// intermediate vectors.
pub struct QuantizationIterator<I, F, Q>
where
    I: Iterator<Item = F>,
    F: Float + NumCast,
    Q: Bounded + NumCast + Copy,
{
    inner: I,
    _phantom_f: core::marker::PhantomData<F>,
    _phantom_q: core::marker::PhantomData<Q>,
}

impl<I, F, Q> QuantizationIterator<I, F, Q>
where
    I: Iterator<Item = F>,
    F: Float + NumCast,
    Q: Bounded + NumCast + Copy,
{
    /// Create a new quantization iterator.
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            _phantom_f: core::marker::PhantomData,
            _phantom_q: core::marker::PhantomData,
        }
    }
}

impl<I, F, Q> Iterator for QuantizationIterator<I, F, Q>
where
    I: Iterator<Item = F>,
    F: Float + NumCast,
    Q: Bounded + NumCast + Copy,
{
    type Item = Q;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        let max_val = Q::max_value();
        let max_val_f: F = NumCast::from(max_val).unwrap_or(F::one());
        let scaled = sample * max_val_f;
        Some(NumCast::from(scaled).unwrap_or(Q::max_value()))
    }
}
