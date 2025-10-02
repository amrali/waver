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

//! Extension traits for easy quantization/dequantization of iterators.

use super::iterators::{DequantizationIterator, QuantizationIterator};
use num_traits::{Bounded, Float, NumCast};

/// Extension trait for easy quantization of iterators.
pub trait QuantizeIterator<F>: Iterator<Item = F>
where
    F: Float + NumCast,
{
    /// Quantize samples in this iterator to the specified type.
    fn quantize<Q>(self) -> QuantizationIterator<Self, F, Q>
    where
        Self: Sized,
        Q: Bounded + NumCast + Copy,
    {
        QuantizationIterator::new(self)
    }
}

impl<I, F> QuantizeIterator<F> for I
where
    I: Iterator<Item = F>,
    F: Float + NumCast,
{
}

/// Extension trait for easy dequantization of iterators.
pub trait DequantizeIterator<Q>: Iterator<Item = Q>
where
    Q: Bounded + NumCast + Copy,
{
    /// Dequantize samples in this iterator to the specified floating-point type.
    fn dequantize<F>(self) -> DequantizationIterator<Self, Q, F>
    where
        Self: Sized,
        F: Float + NumCast,
    {
        DequantizationIterator::new(self)
    }
}

impl<I, Q> DequantizeIterator<Q> for I
where
    I: Iterator<Item = Q>,
    Q: Bounded + NumCast + Copy,
{
}
