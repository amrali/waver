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

//! Core quantization functions.

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
