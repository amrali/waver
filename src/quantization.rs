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

//! A module for sample quantization and conversion utilities.

use alloc::vec::Vec;
use num_traits::{Bounded, Float, NumCast};

/// Convert floating-point samples to integer quantization depth.
///
/// This function takes a slice of floating-point samples (assumed to be in the range [-1.0, 1.0])
/// and quantizes them to the specified integer type's full range.
///
/// # Examples
///
/// ```
/// use waver::quantization::quantize_samples;
///
/// let float_samples = vec![0.0, 0.5, -0.5, 1.0, -1.0];
/// let quantized: Vec<i16> = quantize_samples(&float_samples);
/// assert_eq!(quantized[0], 0);
/// assert!(quantized[1] > 0);
/// assert!(quantized[2] < 0);
/// ```
pub fn quantize_samples<F, Q>(samples: &[F]) -> Vec<Q>
where
    F: Float + NumCast,
    Q: Bounded + NumCast + Copy,
{
    let max_val = Q::max_value();
    let max_val_f: F = NumCast::from(max_val).unwrap_or(F::one());
    samples
        .iter()
        .map(|&sample| {
            let scaled = sample * max_val_f;
            NumCast::from(scaled).unwrap_or(Q::max_value())
        })
        .collect()
}

/// Convert integer samples to floating-point representation.
///
/// This function takes a slice of integer samples and normalizes them to floating-point
/// values in the range [-1.0, 1.0] (for signed integers) or [0.0, 1.0] (for unsigned integers).
///
/// # Examples
///
/// ```
/// use waver::quantization::dequantize_samples;
///
/// let int_samples = vec![0i16, 16384, -16384, 32767, -32768];
/// let dequantized: Vec<f32> = dequantize_samples(&int_samples);
/// assert_eq!(dequantized[0], 0.0);
/// assert!(dequantized[1] > 0.0);
/// assert!(dequantized[2] < 0.0);
/// ```
pub fn dequantize_samples<Q, F>(samples: &[Q]) -> Vec<F>
where
    Q: Bounded + NumCast + Copy,
    F: Float + NumCast,
{
    let max_val = Q::max_value();
    let max_val_f: F = NumCast::from(max_val).unwrap_or(F::one());

    samples
        .iter()
        .map(|&sample| {
            let sample_f: F = NumCast::from(sample).unwrap_or(F::zero());
            sample_f / max_val_f
        })
        .collect()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;

    #[test]
    fn test_quantize_samples() {
        let float_samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];
        let quantized: Vec<i16> = quantize_samples(&float_samples);

        assert_eq!(quantized[0], 0);
        assert!(quantized[1] > 0);
        assert!(quantized[2] < 0);
        assert_eq!(quantized[3], i16::MAX);
        assert!(quantized[4] < 0); // Should be close to i16::MIN but may not be exactly due to scaling
    }

    #[test]
    fn test_dequantize_samples() {
        let int_samples = vec![0i16, 16384, -16384, i16::MAX, i16::MIN];
        let dequantized: Vec<f32> = dequantize_samples(&int_samples);

        assert_eq!(dequantized[0], 0.0);
        assert!(dequantized[1] > 0.0);
        assert!(dequantized[2] < 0.0);
        assert!(dequantized[3] <= 1.0);
        // i16::MIN (-32768) dequantized gives slightly more negative than -1.0 due to asymmetric range
        assert!(dequantized[4] >= -1.0001); // Allow for small precision error beyond -1.0
    }

    #[test]
    fn test_dequantization_iterator() {
        let int_samples = vec![0i16, 16384, -16384, i16::MAX, i16::MIN];
        let dequantized: Vec<f32> = int_samples.iter().copied().dequantize().collect();

        assert_eq!(dequantized[0], 0.0);
        assert!(dequantized[1] > 0.0);
        assert!(dequantized[2] < 0.0);
        assert!(dequantized[3] <= 1.0);
        assert!(dequantized[4] >= -1.0001); // Allow for small precision error beyond -1.0
    }

    #[test]
    fn test_quantize_dequantize_chain() {
        // Test chaining: float -> int -> float
        let original = vec![0.0f32, 0.3, -0.7, 0.9, -0.1];
        
        // Using iterators to chain quantization and dequantization
        let round_trip: Vec<f32> = original
            .iter()
            .copied()
            .quantize::<i16>()
            .dequantize()
            .collect();

        // Should be approximately equal (within quantization error)
        for (orig, round) in original.iter().zip(round_trip.iter()) {
            let error = (orig - round).abs();
            assert!(
                error < 0.01,
                "Round-trip error too large: {} vs {}",
                orig,
                round
            );
        }
    }

    #[test]
    fn test_quantization_iterator() {
        let float_samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];
        let quantized: Vec<i16> = float_samples.iter().copied().quantize().collect();

        assert_eq!(quantized[0], 0);
        assert!(quantized[1] > 0);
        assert!(quantized[2] < 0);
        assert_eq!(quantized[3], i16::MAX);
    }

    #[test]
    fn test_quantization_different_types() {
        // Test f32 -> i8
        let float_samples = vec![0.0f32, 0.25, -0.25, 0.75, -0.75];
        let i8_samples: Vec<i8> = quantize_samples(&float_samples);
        assert_eq!(i8_samples[0], 0);

        // Test f64 -> u16
        let f64_samples = vec![0.0f64, 0.5, 1.0];
        let u16_samples: Vec<u16> = quantize_samples(&f64_samples);
        assert_eq!(u16_samples[0], 0);
        assert_eq!(u16_samples[2], u16::MAX);

        // Test f32 -> f32 (identity conversion)
        let f32_samples = vec![0.0f32, 0.5, -0.5];
        let f32_result: Vec<f32> = quantize_samples(&f32_samples);
        // For f32 -> f32, scaling by f32::MAX produces very large numbers
        assert!(f32_result.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn test_round_trip_conversion() {
        let original = vec![0.0f32, 0.3, -0.7, 0.9, -0.1];
        let quantized: Vec<i16> = quantize_samples(&original);
        let dequantized: Vec<f32> = dequantize_samples(&quantized);

        // Should be approximately equal (within quantization error)
        for (orig, deq) in original.iter().zip(dequantized.iter()) {
            let error = (orig - deq).abs();
            assert!(
                error < 0.01,
                "Round-trip error too large: {} vs {}",
                orig,
                deq
            );
        }
    }
}
