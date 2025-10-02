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

use super::*;

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
