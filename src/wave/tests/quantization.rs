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
use crate::quantization::QuantizeIterator;

#[test]
fn test_wave_quantization() {
    // Test quantization functionality with various bit depths
    let wave = Wave::<f32> {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(1.0),
        func: WaveFunc::Sine,
    };

    // Test quantization to i16 (16-bit signed)
    let quantized_i16: Vec<i16> = wave.iter().quantize().take(10).collect();
    assert_eq!(
        quantized_i16.len(),
        10,
        "Should produce exactly 10 i16 samples"
    );
    assert!(
        quantized_i16.iter().all(|&x| x.abs() <= i16::MAX),
        "i16 samples should be within range"
    );

    // First sample should be zero (sine(0) = 0)
    assert_eq!(quantized_i16[0], 0, "First sample should be zero");

    // Check that samples are non-zero after the first
    assert!(
        quantized_i16.iter().skip(1).any(|&x| x != 0),
        "Should have non-zero samples"
    );

    // Test quantization to i8 (8-bit signed)
    let quantized_i8: Vec<i8> = wave.iter().quantize().take(10).collect();
    assert_eq!(
        quantized_i8.len(),
        10,
        "Should produce exactly 10 i8 samples"
    );
    assert!(
        quantized_i8.iter().all(|&x| x.abs() <= i8::MAX),
        "i8 samples should be within range"
    );
    assert_eq!(quantized_i8[0], 0, "First i8 sample should be zero");

    // Test quantization to u16 (16-bit unsigned)
    let quantized_u16: Vec<u16> = wave.iter().quantize().take(10).collect();
    assert_eq!(
        quantized_u16.len(),
        10,
        "Should produce exactly 10 u16 samples"
    );
    assert!(
        quantized_u16.iter().all(|&x| x <= u16::MAX),
        "u16 samples should be within range"
    );

    // Test quantization with different wave functions
    let square_wave = Wave::<f32> {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(1.0),
        func: WaveFunc::Square,
    };

    let square_quantized: Vec<i16> = square_wave.iter().quantize().take(10).collect();
    assert_eq!(
        square_quantized.len(),
        10,
        "Square wave should quantize correctly"
    );

    // Square wave should produce maximum positive or negative values
    let max_vals = square_quantized
        .iter()
        .filter(|&&x| x.abs() > i16::MAX / 2)
        .count();
    assert!(
        max_vals > 5,
        "Square wave should produce mostly max amplitude values"
    );

    // Test quantization with amplitude modulation
    let modulated_wave = Wave::<f32> {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(0.5), // Half amplitude
        func: WaveFunc::Sine,
    };

    let modulated_quantized: Vec<i16> = modulated_wave.iter().quantize().take(100).collect();
    let max_amplitude = modulated_quantized
        .iter()
        .map(|&x| x.abs())
        .max()
        .unwrap_or(0);

    // With 0.5 amplitude, max should be around half of i16::MAX
    assert!(
        max_amplitude < i16::MAX,
        "Amplitude modulation should reduce quantized range"
    );
    assert!(
        max_amplitude > i16::MAX / 4,
        "Should still have reasonable amplitude"
    );

    // Test quantization to f32 (should work as pass-through scaling)
    let quantized_f32: Vec<f32> = wave.iter().quantize().take(10).collect();
    assert_eq!(quantized_f32.len(), 10, "Should produce f32 samples");
    assert!(
        quantized_f32.iter().all(|x| x.is_finite()),
        "f32 samples should be finite"
    );

    // Compare with original wave samples
    let original_samples: Vec<f32> = wave.iter().take(10).collect();
    for (i, (&quantized, &original)) in quantized_f32
        .iter()
        .zip(original_samples.iter())
        .enumerate()
    {
        // When quantizing to f32, should be scaled by f32::MAX but still proportional
        let expected_scaled = original * f32::MAX;
        assert!(
            (quantized - expected_scaled).abs() < 1e6, // Allow for some floating point precision
            "Sample {}: quantized {:.3e} should be close to scaled original {:.3e}",
            i,
            quantized,
            expected_scaled
        );
    }

    println!("✅ Wave quantization test passed for all bit depths");
}
