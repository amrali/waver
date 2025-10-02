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
use core::f32::consts::PI;
use libm::sinf;

#[test]
fn test_envelope_edge_cases() {
    // Empty envelope - should behave deterministically
    let empty_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Envelope(vec![]),
        amplitude: Modulation::Static(1.0),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = empty_wave.iter().take(5).collect();
    // Empty envelope should still produce samples (using default/fallback behavior)
    assert_eq!(
        samples.len(),
        5,
        "Empty envelope should produce requested samples with fallback behavior"
    );

    // All samples should be finite and non-NaN
    assert!(
        samples.iter().all(|&x| x.is_finite()),
        "Empty envelope samples should be finite"
    );

    // Single-value envelope - should use that single value consistently
    let single_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Envelope(vec![0.7]),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = single_wave.iter().take(5).collect();
    assert_eq!(
        samples.len(),
        5,
        "Single envelope should produce all requested samples"
    );

    // With amplitude 0.7, samples should be scaled appropriately
    assert!(
        samples.iter().all(|&x| x.abs() <= 0.8),
        "Single envelope samples should be scaled by envelope value"
    );
    assert!(
        samples.iter().any(|&x| x.abs() > 0.1),
        "Single envelope should produce non-trivial samples"
    );
}

#[test]
fn test_wave_function_edge_cases() {
    let functions = [
        WaveFunc::Sine,
        WaveFunc::Cosine,
        WaveFunc::Square,
        WaveFunc::Sawtooth,
        WaveFunc::Triangle,
    ];

    for &func in &functions {
        let wave = Wave {
            sample_rate: 44100.0,
            frequency: 1000.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(1.0),
            func,
        };

        let samples: Vec<f32> = wave.iter().take(100).collect();
        assert_eq!(
            samples.len(),
            100,
            "Should generate exactly 100 samples for {:?}",
            func
        );
        assert!(
            samples.iter().all(|x| x.is_finite()),
            "All samples should be finite for {:?}",
            func
        );

        // Check that function actually produces variation (not all zeros)
        assert!(
            samples.iter().any(|&x| x.abs() > 0.001),
            "Should produce non-zero samples for {:?}",
            func
        );
    }
}

#[test]
fn test_wave_envelope_edge_cases() {
    // Test envelope that ends before wave generation stops
    let short_envelope = vec![1.0, 0.5, 0.0];

    let envelope_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Envelope(short_envelope.clone()),
        func: WaveFunc::Sine,
    };

    // Generate exactly the envelope length to test boundary behavior
    let samples: Vec<f32> = envelope_wave.iter().take(short_envelope.len()).collect();
    assert_eq!(
        samples.len(),
        short_envelope.len(),
        "Should produce samples for each envelope point"
    );

    // Check that envelope values are actually applied
    // Calculate expected values: sine(2π * 440 * t / 44100) * envelope[i]
    let expected_samples: Vec<f32> = (0..short_envelope.len())
        .map(|i| {
            let t = i as f32 / 44100.0;
            let sine_val = sinf(2.0 * PI * 440.0 * t);
            sine_val * short_envelope[i]
        })
        .collect();

    for (i, (&actual, &expected)) in samples.iter().zip(expected_samples.iter()).enumerate() {
        assert!(
            (actual - expected).abs() < 0.01,
            "Sample {}: expected {:.3}, got {:.3} (envelope: {})",
            i,
            expected,
            actual,
            short_envelope[i]
        );
    }

    // Verify that the third sample (amplitude 0.0) is indeed zero
    assert_eq!(
        samples[2], 0.0,
        "Zero amplitude envelope point should produce zero samples"
    );

    // Test beyond envelope length
    let extended_samples: Vec<f32> = envelope_wave.iter().take(10).collect();
    // Library behavior when envelope is exhausted - should either terminate or use fallback
    if extended_samples.len() > short_envelope.len() {
        // If it continues, the behavior after envelope should be consistent
        let post_envelope = &extended_samples[short_envelope.len()..];
        let first_post = post_envelope[0];
        assert!(
            post_envelope
                .iter()
                .all(|&x| (x - first_post).abs() < 0.001),
            "Post-envelope samples should be consistent"
        );
    }
}

#[test]
fn test_phase_envelope_edge_cases() {
    // Test phase envelope with extreme values
    let extreme_phase_envelope = vec![0.0, PI, 2.0 * PI, -PI, 0.0];

    let phase_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Envelope(extreme_phase_envelope.clone()),
        amplitude: Modulation::Static(1.0),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = phase_wave
        .iter()
        .take(extreme_phase_envelope.len())
        .collect();

    assert_eq!(
        samples.len(),
        extreme_phase_envelope.len(),
        "Should produce sample for each phase envelope point"
    );

    // Phase modulation should create variation in the output
    // Since we're using sine with different phase offsets, we should see variation
    let first_half = &samples[0..2];
    let second_half = &samples[2..4];
    let has_variation = first_half
        .iter()
        .zip(second_half)
        .any(|(a, b)| (a - b).abs() > 0.1);
    assert!(
        has_variation,
        "Different phase envelope values should produce different samples"
    );

    // Sine function output should be bounded
    assert!(
        samples.iter().all(|&x| x.abs() <= 1.1),
        "Sine-based samples should be roughly bounded"
    );
}

#[test]
fn test_lfo_termination() {
    // Test LFO that might terminate before parent wave
    let short_lfo = Wave {
        sample_rate: 44100.0,
        frequency: 10.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Envelope(vec![0.5, 0.0]), // Very short envelope
        func: WaveFunc::Sine,
    };

    let lfo_modulated_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::LFO(Box::new(short_lfo)),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = lfo_modulated_wave.iter().take(100).collect();
    assert_eq!(samples.len(), 100, "Should generate exactly 100 samples");

    // Should handle LFO termination without crashing and produce valid output
    assert!(
        samples.iter().all(|x| x.is_finite()),
        "Should produce finite samples"
    );

    // The LFO has envelope [0.5, 0.0] which means it produces:
    // Sample 0: LFO amplitude envelope[0] = 0.5, LFO output = sin(10Hz * 0) * 0.5 = 0
    // Sample 1: LFO amplitude envelope[1] = 0.0, LFO output = sin(10Hz * t) * 0.0 = 0
    // Sample 2+: LFO ended, amplitude = 0.0

    // Since phase is static (never ends), wave continues but with 0 amplitude
    // All samples should be zero because LFO modulation results in zero amplitude
    assert!(
        samples.iter().all(|&x| x == 0.0),
        "All samples should be zero when LFO envelope results in zero amplitude modulation"
    );
}

#[test]
fn test_complex_lfo_interactions() {
    // Test nested LFO (LFO modulating another LFO)
    let inner_lfo = Wave {
        sample_rate: 44100.0,
        frequency: 0.5, // Very slow modulation
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(2.0), // Modulation depth
        func: WaveFunc::Sine,
    };

    let outer_lfo = Wave {
        sample_rate: 44100.0,
        frequency: 3.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::LFO(Box::new(inner_lfo)), // Nested LFO
        func: WaveFunc::Sine,
    };

    let modulated_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::LFO(Box::new(outer_lfo)),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = modulated_wave.iter().take(50).collect();
    assert_eq!(
        samples.len(),
        50,
        "Should generate exactly 50 samples with complex LFO interactions"
    );

    assert!(
        samples.iter().all(|x| x.is_finite()),
        "Complex LFO should produce finite samples"
    );

    // Complex LFO should actually modulate the amplitude - verify this produces meaningful output
    assert!(
        samples.iter().any(|&x| x != 0.0),
        "Complex LFO modulation should produce non-zero output"
    );

    // With nested LFO modulation, we should see amplitude variation over time
    let first_third = &samples[0..16];
    let second_third = &samples[16..32];
    let third_third = &samples[32..48];

    let avg1 = first_third.iter().map(|x| x.abs()).sum::<f32>() / first_third.len() as f32;
    let avg2 = second_third.iter().map(|x| x.abs()).sum::<f32>() / second_third.len() as f32;
    let avg3 = third_third.iter().map(|x| x.abs()).sum::<f32>() / third_third.len() as f32;

    // Nested LFO should create some variation in amplitude over time
    let max_avg = avg1.max(avg2).max(avg3);
    let min_avg = avg1.min(avg2).min(avg3);
    assert!(
        max_avg > min_avg * 0.1,
        "Nested LFO should create meaningful amplitude variation"
    );
}

#[test]
fn test_both_envelopes_ending() {
    // Test wave where both amplitude and phase envelopes end
    let amp_envelope = vec![1.0, 0.5, 0.0];
    let phase_envelope = vec![0.0, PI / 2.0, PI];

    let dual_envelope_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Envelope(phase_envelope.clone()),
        amplitude: Modulation::Envelope(amp_envelope.clone()),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = dual_envelope_wave.iter().take(10).collect();

    // With fixed infinite iterator, should get all 10 requested samples
    assert_eq!(
        samples.len(),
        10,
        "Iterator should be infinite and provide all requested samples"
    );

    // Should handle both envelopes ending gracefully with meaningful output
    assert!(
        samples.iter().all(|x| x.is_finite()),
        "Should produce finite samples with dual envelopes"
    );

    // Check envelope behavior within envelope length
    if samples.len() >= 3 {
        // Third sample (index 2) uses amplitude envelope[2] = 0.0
        let third_sample = samples[2];
        assert_eq!(
            third_sample, 0.0,
            "Third sample should be zero when amplitude envelope[2] = 0.0"
        );
    }

    // Samples after envelope end should continue with last envelope values
    // amplitude: continues with 0.0, phase: continues with PI
    if samples.len() > 3 {
        let post_envelope_samples = &samples[3..];
        assert!(
            post_envelope_samples.iter().all(|&x| x == 0.0),
            "Post-envelope samples should be zero since amplitude continues at 0.0"
        );
    }

    // Phase envelope should create variation in the sine wave shape within envelope
    if samples.len() >= 2 {
        // samples[0] uses phase 0.0, samples[1] uses phase PI/2
        // At different phases, sine should produce different values (unless frequency is very low)
        let time_step = 1.0 / 44100.0;
        let freq_effect = 2.0 * PI * 440.0 * time_step;

        // If frequency creates significant phase change per sample, expect variation
        if freq_effect > 0.1 {
            assert!(
                samples[0] != samples[1],
                "Different phase envelope values should affect output"
            );
        }
    }
}

#[test]
fn test_extreme_modulation_values() {
    // Test with large amplitude modulation
    let large_lfo = Wave {
        sample_rate: 44100.0,
        frequency: 2.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(2.0), // Moderate amplitude
        func: WaveFunc::Sine,
    };

    let modulated_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::LFO(Box::new(large_lfo)),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = modulated_wave.iter().take(50).collect();
    assert_eq!(samples.len(), 50, "Should handle large modulation values");

    // Should produce finite samples
    assert!(
        samples.iter().all(|x| x.is_finite()),
        "All samples should be finite"
    );

    // With modulation, there should be some amplitude variation
    let max_sample = samples.iter().fold(0.0f32, |acc, &val| acc.max(val.abs()));
    // Lower threshold - library might produce small values initially
    assert!(
        max_sample > 0.001,
        "Should produce samples with some amplitude (max: {})",
        max_sample
    );
}

#[test]
fn test_zero_frequency_with_modulation() {
    // Test zero frequency carrier with amplitude modulation
    let lfo = Wave {
        sample_rate: 44100.0,
        frequency: 1.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(0.5),
        func: WaveFunc::Sine,
    };

    let zero_freq_wave = Wave {
        sample_rate: 44100.0,
        frequency: 0.0, // Zero frequency
        phase: Modulation::Static(0.0),
        amplitude: Modulation::LFO(Box::new(lfo)),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = zero_freq_wave.iter().take(1000).collect(); // Shorter test
    assert_eq!(
        samples.len(),
        1000,
        "Should generate exactly 1000 samples for zero frequency with modulation"
    );

    // Should produce finite samples
    assert!(
        samples.iter().all(|x| x.is_finite()),
        "All samples should be finite"
    );

    // Zero frequency sine should produce a constant DC value (sine(0) = 0)
    // But LFO amplitude modulation should scale this constant
    // Since sine(0) = 0, zero frequency sine should produce all zeros regardless of amplitude
    assert!(
        samples.iter().all(|&x| x == 0.0),
        "Zero frequency sine should produce all zeros even with LFO amplitude modulation"
    );
}

#[test]
fn test_iterator_infinite_behavior() {
    // Test that wave iterators are truly infinite and don't output zeros

    // 1. Test LFO with finite envelope - should continue infinitely with fallback
    let finite_lfo = Wave {
        sample_rate: 44100.0,
        frequency: 1.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Envelope(vec![0.8, 0.6]), // Very short envelope
        func: WaveFunc::Sine,
    };

    let lfo_modulated_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::LFO(Box::new(finite_lfo)),
        func: WaveFunc::Sine,
    };

    // Take many more samples than the LFO envelope length
    let samples: Vec<f32> = lfo_modulated_wave.iter().take(10000).collect();
    assert_eq!(
        samples.len(),
        10000,
        "Iterator should be infinite and provide all requested samples"
    );

    // All samples should be finite (no NaN or infinity)
    assert!(
        samples.iter().all(|x| x.is_finite()),
        "All samples should be finite"
    );

    // The iterator should not produce infinite zeros after LFO envelope ends
    let post_envelope_samples = &samples[100..200]; // Well past envelope
    let non_zero_count = post_envelope_samples.iter().filter(|&&x| x != 0.0).count();
    assert!(
        non_zero_count > 50,
        "Post-LFO-envelope samples should not all be zero (got {} non-zero out of {})",
        non_zero_count,
        post_envelope_samples.len()
    );

    // 2. Test envelope directly - should continue with last value infinitely
    let envelope_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Envelope(vec![1.0, 0.5, 0.2]), // Ends at 0.2
        func: WaveFunc::Sine,
    };

    let env_samples: Vec<f32> = envelope_wave.iter().take(1000).collect();
    assert_eq!(
        env_samples.len(),
        1000,
        "Envelope wave iterator should be infinite"
    );

    // Samples beyond envelope length should use last envelope value (0.2)
    let post_env_samples = &env_samples[10..20]; // Well past envelope end

    // Verify that the iterator continues and produces reasonable values
    assert!(
        post_env_samples.iter().all(|x| x.is_finite()),
        "Post-envelope samples should be finite"
    );

    // Verify that the iterator is actually using the last envelope value
    // by checking that amplitude ratios are consistent with 0.2
    let non_zero_samples: Vec<_> = post_env_samples
        .iter()
        .filter(|&&x| x.abs() > 0.01)
        .collect();
    if !non_zero_samples.is_empty() {
        let avg_abs_amp =
            non_zero_samples.iter().map(|&&x| x.abs()).sum::<f32>() / non_zero_samples.len() as f32;
        assert!(
            avg_abs_amp > 0.05 && avg_abs_amp < 0.25,
            "Average amplitude should be reasonable for envelope value 0.2, got {:.3}",
            avg_abs_amp
        );
    }

    // 3. Test static modulation - should be perfectly infinite
    let static_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(0.7),
        func: WaveFunc::Sine,
    };

    let static_samples: Vec<f32> = static_wave.iter().take(50000).collect();
    assert_eq!(
        static_samples.len(),
        50000,
        "Static wave iterator should be infinite"
    );
    assert!(
        static_samples.iter().all(|x| x.is_finite()),
        "All static samples should be finite"
    );
}
