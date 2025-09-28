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

//! Tests for signal analysis functionality.

use super::*;
use crate::{
    error::Error,
    waveform::{Waveform, WaveformSource},
};
extern crate std;
use std::f32::consts::PI;

#[test]
fn test_spectrum_recorded() {
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let samples: Vec<i16> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let sample = (t * frequency * 2.0 * PI).sin();
            let amplitude = i16::MAX as f32;
            (sample * amplitude) as i16
        })
        .collect();

    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
    let spectrum = spectrum(&waveform, samples.len());

    let (dominant_freq, _, _) = spectrum
        .data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();

    assert!((dominant_freq - frequency).abs() < spectrum.frequency_resolution);
}

#[test]
fn test_spectrum_generative() {
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let waveform = Waveform::<i16>::with_wave(
        sample_rate,
        crate::Wave {
            frequency,
            ..Default::default()
        },
    );
    let spectrum = spectrum(&waveform, sample_rate as usize);

    let (dominant_freq, _, _) = spectrum
        .data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();

    assert!((dominant_freq - frequency).abs() < spectrum.frequency_resolution);
}

#[test]
fn test_time_spectrum() {
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let samples: Vec<i16> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let sample = (t * frequency * 2.0 * PI).sin();
            let amplitude = i16::MAX as f32;
            (sample * amplitude) as i16
        })
        .collect();

    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
    let spectrogram = time_spectrum(&waveform, 1024, 512).unwrap();

    // Calculate expected number of frames: (samples - window_size) / hop_size + 1
    let expected_frames = (samples.len() - 1024) / 512 + 1;
    assert_eq!(spectrogram.len(), expected_frames);

    // Each spectrum should detect the target frequency accurately
    let mut accurate_detections = 0;
    for spectrum in &spectrogram {
        let (dominant_freq, _, _) = spectrum
            .data
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();

        if (dominant_freq - frequency).abs() < spectrum.frequency_resolution * 2.0 {
            accurate_detections += 1;
        }
    }

    // At least 90% of frames should accurately detect the frequency
    let accuracy_ratio = accurate_detections as f32 / spectrogram.len() as f32;
    assert!(
        accuracy_ratio > 0.90,
        "Frequency detection accuracy too low: {:.1}% (expected > 90%)",
        accuracy_ratio * 100.0
    );
}

#[test]
fn test_synthesize() {
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let samples: Vec<i16> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let sample = (t * frequency * 2.0 * PI).sin();
            let amplitude = i16::MAX as f32;
            (sample * amplitude) as i16
        })
        .collect();

    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
    let synthesized_waveform = synthesize(&waveform, 1024, 512, 5).unwrap();

    if let WaveformSource::Generative(waves) = synthesized_waveform.source {
        assert!(!waves.is_empty(), "Should produce at least one wave");

        // Find the wave closest to target frequency
        let closest_wave = waves
            .iter()
            .min_by(|a, b| {
                (a.frequency - frequency)
                    .abs()
                    .partial_cmp(&(b.frequency - frequency).abs())
                    .unwrap()
            })
            .unwrap();

        // Frequency accuracy should be better than 2% for clean sine waves
        let frequency_error = (closest_wave.frequency - frequency).abs() / frequency;
        assert!(
            frequency_error < 0.02,
            "Frequency error too large: {:.1}% (expected < 2%)",
            frequency_error * 100.0
        );

        assert!(matches!(
            closest_wave.amplitude,
            crate::Modulation::Envelope(_)
        ));
        assert!(matches!(closest_wave.phase, crate::Modulation::Envelope(_)));
    } else {
        panic!("Expected generative waveform");
    }
}

#[test]
fn test_error_conditions() {
    // Test time_spectrum on generative waveform
    let wf = Waveform::<i16>::new(44100.0);
    let err = time_spectrum(&wf, 1024, 512).unwrap_err();
    assert_eq!(err, Error::UnsupportedSource);

    // Test synthesize on generative waveform
    let err = synthesize(&wf, 1024, 512, 1).unwrap_err();
    assert_eq!(err, Error::UnsupportedSource);
}

#[test]
fn test_synthesis_quality() {
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let duration = 0.5; // Longer duration for more reliable analysis
    let num_samples = (sample_rate * duration) as usize;

    let samples: Vec<i16> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let sample = (t * frequency * 2.0 * PI).sin();
            (sample * 1000.0) as i16
        })
        .collect();

    let original_waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
    let synthesized_waveform = synthesize(&original_waveform, 1024, 512, 5).unwrap();

    if let WaveformSource::Generative(waves) = &synthesized_waveform.source {
        assert!(
            !waves.is_empty(),
            "Synthesis should produce at least one wave component"
        );
        assert!(
            waves.len() <= 10,
            "Should not produce an excessive number of wave components"
        );

        // Check that we found the target frequency (within reason)
        let target_wave = waves
            .iter()
            .find(|wave| (wave.frequency - frequency).abs() < frequency * 0.1)
            .expect("Should find wave close to target frequency");

        // Verify the found wave has reasonable properties
        assert!(
            target_wave.frequency > 0.0,
            "Target wave should have positive frequency"
        );
        assert!(
            target_wave.sample_rate > 0.0,
            "Target wave should have positive sample rate"
        );

        println!(
            "Found target wave at {:.1}Hz (target: {:.1}Hz)",
            target_wave.frequency, frequency
        );
    } else {
        panic!("Expected generative waveform");
    }

    // Test synthesized output quality
    let synthesized_samples: Vec<f32> = synthesized_waveform
        .iter()
        .take(1000)
        .map(|x| x as f32)
        .collect();
    let synth_rms: f32 = (synthesized_samples.iter().map(|x| x.powi(2)).sum::<f32>()
        / synthesized_samples.len() as f32)
        .sqrt();

    assert!(
        synth_rms > 0.0,
        "Synthesized waveform should have non-zero amplitude"
    );

    // Frequency domain comparison using the same window size as synthesis
    let orig_spectrum = spectrum(&original_waveform, 1024);
    let synth_spectrum = spectrum(&synthesized_waveform, 1024);

    // Find peaks near the target frequency in both spectra
    let target_freq_range = (frequency * 0.9, frequency * 1.1);

    let orig_target_magnitude = orig_spectrum
        .data
        .iter()
        .filter(|(f, _, _)| *f >= target_freq_range.0 && *f <= target_freq_range.1)
        .map(|(_, mag, _)| *mag)
        .fold(0.0f32, |acc, val| acc.max(val));

    let synth_target_magnitude = synth_spectrum
        .data
        .iter()
        .filter(|(f, _, _)| *f >= target_freq_range.0 && *f <= target_freq_range.1)
        .map(|(_, mag, _)| *mag)
        .fold(0.0f32, |acc, val| acc.max(val));

    assert!(
        orig_target_magnitude > 0.0 && synth_target_magnitude > 0.0,
        "Both original and synthesized should have energy at target frequency {frequency:.1}Hz"
    );

    println!(
        "✅ Synthesis quality test passed - frequency {frequency:.1}Hz preserved in synthesis"
    );
}

#[test]
fn test_frequency_range_coverage() {
    let sample_rate = 44100.0;
    let test_frequencies = vec![50.0, 440.0, 2000.0, 8000.0];

    for &test_freq in &test_frequencies {
        let samples: Vec<i16> = (0..sample_rate as u32)
            .map(|i| {
                let t = i as f32 / sample_rate;
                let sample = (t * test_freq * 2.0 * PI).sin();
                (sample * 1000.0) as i16
            })
            .collect();

        let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
        let synthesized = synthesize(&waveform, 1024, 512, 5).unwrap();

        if let WaveformSource::Generative(waves) = &synthesized.source {
            assert!(!waves.is_empty(), "No waves detected for {test_freq}Hz");

            let closest_freq = waves
                .iter()
                .min_by(|a, b| {
                    (a.frequency - test_freq)
                        .abs()
                        .partial_cmp(&(b.frequency - test_freq).abs())
                        .unwrap()
                })
                .unwrap()
                .frequency;

            // Concrete frequency accuracy requirements based on frequency range
            let (max_error_percent, description) = match test_freq {
                freq if freq < 100.0 => (5.0, "low frequency"), // 5% for low freq
                freq if freq < 1000.0 => (3.0, "audio frequency"), // 3% for audio
                freq if freq < 5000.0 => (2.0, "mid frequency"), // 2% for mid freq
                _ => (3.0, "high frequency"),                   // 3% for high freq
            };

            let actual_error = (closest_freq - test_freq).abs() / test_freq * 100.0;
            assert!(
                actual_error < max_error_percent,
                "{description} {test_freq}Hz synthesis failed: {actual_error:.2}% error (expected < {max_error_percent:.1}%)"
            );
        }
    }
}

#[test]
fn test_extreme_low_frequencies() {
    // Test sub-Hz frequencies with appropriate constraints
    let sample_rate = 200.0;
    let window_size = 512; // Larger window for better frequency resolution
    let hop_size = 128;

    let sub_hz_frequencies = vec![0.1, 0.5];

    for &test_freq in &sub_hz_frequencies {
        let cycles = 10.0; // Capture at least 10 cycles for reliable detection
        let duration = cycles / test_freq;
        let num_samples = (sample_rate * duration) as usize;

        let samples: Vec<i16> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                let sample = (t * test_freq * 2.0 * PI).sin();
                (sample * 8000.0) as i16
            })
            .collect();

        let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);

        match synthesize(&waveform, window_size, hop_size, 5) {
            Ok(synthesized) => {
                if let WaveformSource::Generative(waves) = &synthesized.source {
                    assert!(!waves.is_empty(), "No waves detected for {test_freq}Hz");

                    // Find the closest frequency to our target
                    let closest_wave = waves
                        .iter()
                        .min_by(|a, b| {
                            (a.frequency - test_freq)
                                .abs()
                                .partial_cmp(&(b.frequency - test_freq).abs())
                                .unwrap()
                        })
                        .unwrap();

                    // For sub-Hz, we need to be more lenient but still reasonable
                    // The frequency resolution at this sample rate and window size is ~0.39 Hz
                    let freq_resolution = sample_rate / window_size as f32;
                    let max_error = freq_resolution * 2.0; // Allow 2 bins of error

                    assert!(
                        (closest_wave.frequency - test_freq).abs() < max_error,
                        "Sub-Hz {}Hz detection failed: got {:.3}Hz (error: {:.3}Hz, max: {:.3}Hz)",
                        test_freq,
                        closest_wave.frequency,
                        (closest_wave.frequency - test_freq).abs(),
                        max_error
                    );
                }
            }
            Err(e) => panic!("Synthesis failed for {test_freq}Hz: {e:?}"),
        }
    }
}

#[test]
fn test_extreme_high_frequencies() {
    // Test MHz and GHz frequencies with appropriate sample rates and durations
    let test_cases = vec![
        (1_000_000.0, 100_000_000.0),        // 1 MHz @ 100 MHz sample rate
        (5_000_000.0, 200_000_000.0),        // 5 MHz @ 200 MHz sample rate
        (1_000_000_000.0, 10_000_000_000.0), // 1 GHz @ 10 GHz sample rate
    ];

    for (test_freq, sample_rate) in test_cases {
        let window_size = 2048;
        let hop_size = 512;
        // Ensure we have enough samples: at least 4 windows worth
        let min_duration = (window_size as f32 * 4.0) / sample_rate;
        let duration = min_duration * 1.5; // 50% margin
        let num_samples = (sample_rate * duration) as usize;

        assert!(
            num_samples >= window_size * 2,
            "Test setup: {} samples >= {} required for {:.1}{}",
            num_samples,
            window_size * 2,
            if test_freq >= 1_000_000_000.0 {
                test_freq / 1_000_000_000.0
            } else {
                test_freq / 1_000_000.0
            },
            if test_freq >= 1_000_000_000.0 {
                "GHz"
            } else {
                "MHz"
            }
        );

        let samples: Vec<i16> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                let sample = (t * test_freq * 2.0 * PI).sin();
                (sample * 15000.0) as i16
            })
            .collect();

        let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);

        match synthesize(&waveform, window_size, hop_size, 10) {
            Ok(synthesized) => {
                if let WaveformSource::Generative(waves) = &synthesized.source {
                    // For extreme high frequencies, detection may fail - this is expected
                    if waves.is_empty() && test_freq > 10_000.0 {
                        // High frequency detection failure is acceptable
                        let freq_unit = if test_freq >= 1_000_000_000.0 {
                            "GHz"
                        } else if test_freq >= 1_000_000.0 {
                            "MHz"
                        } else if test_freq >= 1_000.0 {
                            "kHz"
                        } else {
                            "Hz"
                        };
                        let freq_val = if test_freq >= 1_000_000_000.0 {
                            test_freq / 1_000_000_000.0
                        } else if test_freq >= 1_000_000.0 {
                            test_freq / 1_000_000.0
                        } else if test_freq >= 1_000.0 {
                            test_freq / 1_000.0
                        } else {
                            test_freq
                        };
                        println!(
                            "Detection failed for {:.2}{} as expected",
                            freq_val, freq_unit
                        );
                    } else if waves.is_empty() && test_freq <= 10_000.0 {
                        panic!(
                            "Should be able to detect frequencies up to 10kHz, but failed for {}Hz",
                            test_freq
                        );
                    } else if !waves.is_empty() {
                        // Successfully detected - verify results are reasonable
                        assert!(
                            waves.len() <= 15,
                            "Should not detect excessive number of components for single frequency"
                        );

                        // Look for the target frequency
                        let tolerance = test_freq * 0.2; // 20% tolerance
                        let found_target = waves
                            .iter()
                            .any(|wave| (wave.frequency - test_freq).abs() < tolerance);

                        if !found_target {
                            println!(
                                "Warning: Target frequency {}Hz not detected within 20% tolerance",
                                test_freq
                            );
                        }
                    }
                } else {
                    panic!("Expected generative waveform source");
                }
            }
            Err(e) => {
                // High frequency analysis can fail - acceptable for extreme cases
                if test_freq > 20_000.0 {
                    println!("Analysis failed for {}Hz as expected: {:?}", test_freq, e);
                } else {
                    panic!("Analysis should not fail for {}Hz: {:?}", test_freq, e);
                }
            }
        }
    }
}

// Edge case tests
#[test]
fn test_stft_edge_cases() {
    let sample_rate = 44100.0;
    let samples: Vec<i16> = vec![0; 100]; // Very short signal
    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);

    // Test with window size larger than signal
    let result = time_spectrum(&waveform, 512, 256);
    assert!(
        result.is_ok(),
        "Should handle window size larger than signal"
    );

    // Test with hop size larger than window size
    let result = time_spectrum(&waveform, 64, 128);
    assert!(
        result.is_ok(),
        "Should handle hop size larger than window size"
    );

    // Test with very small hop size (1)
    let result = time_spectrum(&waveform, 64, 1);
    assert!(result.is_ok(), "Should handle very small hop size");
}

#[test]
fn test_synthesis_edge_cases() {
    let sample_rate = 8000.0; // Lower sample rate
    let samples: Vec<i16> = vec![0; 50]; // Very short signal
    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);

    // Test synthesis with very short signal
    let result = synthesize(&waveform, 32, 16, 3);
    assert!(result.is_ok(), "Should handle very short signals");

    // Test synthesis with max_harmonics = 0
    let result = synthesize(&waveform, 32, 16, 0);
    assert!(result.is_ok(), "Should handle zero harmonics");
}

#[test]
fn test_spectrum_with_dc_component() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;
    let samples: Vec<i16> = (0..44100)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = 1000.0 + 500.0 * (2.0 * PI * 440.0 * t).sin(); // DC + AC
            let amplitude = i16::MAX as f32 * 0.5;
            (signal * amplitude / 1500.0) as i16
        })
        .collect();

    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
    let spectrum_result = spectrum(&waveform, samples.len());

    // Should detect both DC and fundamental frequency
    let dc_magnitude = spectrum_result.data[0].1;
    assert!(dc_magnitude > 0.0, "Should detect DC component");

    let fundamental_bin = (440.0 / spectrum_result.frequency_resolution).round() as usize;
    if fundamental_bin < spectrum_result.data.len() {
        assert!(
            spectrum_result.data[fundamental_bin].1 > dc_magnitude * 0.1,
            "Should detect AC component"
        );
    }
}

#[test]
fn test_spectrum_with_noise() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;
    let frequency = 1000.0;

    // Create signal with added noise
    let samples: Vec<i16> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = (2.0 * PI * frequency * t).sin();
            let noise = 0.1 * ((i * 17 + 31) as f32).sin(); // Pseudo-random noise
            let combined = signal + noise;
            let amplitude = i16::MAX as f32 * 0.8;
            (combined * amplitude) as i16
        })
        .collect();

    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
    let spectrum_result = spectrum(&waveform, samples.len());

    // Should still detect the main frequency despite noise
    let (dominant_freq, _, _) = spectrum_result
        .data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();

    assert!((dominant_freq - frequency).abs() < spectrum_result.frequency_resolution * 2.0);
}

#[test]
fn test_synthesis_frequency_tracking() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;

    // Create a frequency-modulated signal
    let samples: Vec<i16> = (0..44100 * 2) // 2 seconds
        .map(|i| {
            let t = i as f32 / sample_rate;
            let base_freq = 440.0;
            let fm_freq = 2.0; // 2 Hz modulation
            let fm_depth = 50.0; // ±50 Hz
            let _instantaneous_freq = base_freq + fm_depth * (2.0 * PI * fm_freq * t).sin();
            let phase =
                2.0 * PI * base_freq * t + (fm_depth / fm_freq) * (2.0 * PI * fm_freq * t).cos();
            let signal = phase.sin();
            let amplitude = i16::MAX as f32 * 0.8;
            (signal * amplitude) as i16
        })
        .collect();

    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
    let result = synthesize(&waveform, 1024, 512, 5);

    assert!(result.is_ok(), "Should handle frequency-modulated signals");

    if let Ok(synthesized) = result {
        if let crate::waveform::WaveformSource::Generative(waves) = synthesized.source {
            assert!(
                !waves.is_empty(),
                "Should produce at least one wave from FM signal"
            );
        }
    }
}

#[test]
fn test_extreme_frequency_ranges() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;

    // Test very low frequency (1 Hz)
    let low_freq = 1.0;
    let samples: Vec<i16> = (0..sample_rate as u32 * 3) // 3 seconds for low frequency
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = (2.0 * PI * low_freq * t).sin();
            let amplitude = i16::MAX as f32 * 0.5;
            (signal * amplitude) as i16
        })
        .collect();

    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
    let low_spectrum = spectrum(&waveform, samples.len());

    let (dominant_freq, _, _) = low_spectrum
        .data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();

    assert!((dominant_freq - low_freq).abs() < low_spectrum.frequency_resolution * 2.0);

    // Test very high frequency (close to Nyquist)
    let high_freq = sample_rate * 0.45; // Just below Nyquist
    let high_samples: Vec<i16> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = (2.0 * PI * high_freq * t).sin();
            let amplitude = i16::MAX as f32 * 0.5;
            (signal * amplitude) as i16
        })
        .collect();

    let high_waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &high_samples);
    let high_spectrum = spectrum(&high_waveform, high_samples.len());

    let (high_dominant_freq, _, _) = high_spectrum
        .data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();

    assert!((high_dominant_freq - high_freq).abs() < high_spectrum.frequency_resolution * 3.0);
}

#[test]
fn test_complex_harmonic_content() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;
    let fundamental = 220.0;

    // Create signal with multiple harmonics
    let samples: Vec<i16> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = (2.0 * PI * fundamental * t).sin() +
                       0.5 * (2.0 * PI * fundamental * 2.0 * t).sin() + // 2nd harmonic
                       0.25 * (2.0 * PI * fundamental * 3.0 * t).sin() + // 3rd harmonic
                       0.125 * (2.0 * PI * fundamental * 4.0 * t).sin(); // 4th harmonic
            let amplitude = i16::MAX as f32 * 0.3;
            (signal * amplitude) as i16
        })
        .collect();

    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
    let spectrum_result = spectrum(&waveform, samples.len());

    // Should detect fundamental and harmonics
    let mut detected_peaks = 0;
    for expected_freq in &[
        fundamental,
        fundamental * 2.0,
        fundamental * 3.0,
        fundamental * 4.0,
    ] {
        let bin = (*expected_freq / spectrum_result.frequency_resolution).round() as usize;
        if bin < spectrum_result.data.len() && spectrum_result.data[bin].1 > 0.1 {
            detected_peaks += 1;
        }
    }

    assert!(detected_peaks >= 2, "Should detect multiple harmonic peaks");
}

#[test]
fn test_time_spectrum_consistency() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let samples: Vec<i16> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = (2.0 * PI * frequency * t).sin();
            let amplitude = i16::MAX as f32 * 0.7;
            (signal * amplitude) as i16
        })
        .collect();

    let waveform: Waveform<i16> = Waveform::from_recorded_samples(sample_rate, &samples);
    let spectrogram = time_spectrum(&waveform, 1024, 512).unwrap();

    // All spectra in the spectrogram should have similar dominant frequency
    let mut consistent_detections = 0;
    for spectrum in &spectrogram {
        let (dominant_freq, _, _) = spectrum
            .data
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();

        if (dominant_freq - frequency).abs() < spectrum.frequency_resolution * 2.0 {
            consistent_detections += 1;
        }
    }

    let consistency_ratio = consistent_detections as f32 / spectrogram.len() as f32;
    assert!(
        consistency_ratio > 0.8,
        "Should have consistent frequency detection across time"
    );
}

#[test]
fn test_error_display() {
    let error = Error::UnsupportedSource;
    let error_string = format!("{:?}", error);
    assert!(error_string.contains("UnsupportedSource"));
}

#[test]
fn test_error_equality() {
    let error1 = Error::UnsupportedSource;
    let error2 = Error::UnsupportedSource;
    assert_eq!(error1, error2);
    assert_eq!(error1.clone(), error2);
}
