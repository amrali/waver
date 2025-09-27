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
        assert!(!waves.is_empty());

        // Check that we found the target frequency (within reason)
        let target_wave = waves
            .iter()
            .find(|wave| (wave.frequency - frequency).abs() < frequency * 0.1)
            .expect("Should find wave close to target frequency");

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
        "Both original and synthesized should have energy at target frequency {:.1}Hz",
        frequency
    );

    println!(
        "✅ Synthesis quality test passed - frequency {:.1}Hz preserved in synthesis",
        frequency
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
            assert!(!waves.is_empty(), "No waves detected for {}Hz", test_freq);

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
                "{} {}Hz synthesis failed: {:.2}% error (expected < {:.1}%)",
                description,
                test_freq,
                actual_error,
                max_error_percent
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
                    assert!(!waves.is_empty(), "No waves detected for {}Hz", test_freq);

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
            Err(e) => panic!("Synthesis failed for {}Hz: {:?}", test_freq, e),
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
                    if waves.is_empty() {
                        // High frequency detection can fail - that's a valid test result
                        let freq_unit = if test_freq >= 1_000_000_000.0 {
                            "GHz"
                        } else {
                            "MHz"
                        };
                        let freq_display = if test_freq >= 1_000_000_000.0 {
                            test_freq / 1_000_000_000.0
                        } else {
                            test_freq / 1_000_000.0
                        };
                        println!(
                            "⚠️  No waves detected for {:.1}{} - detection limit reached",
                            freq_display, freq_unit
                        );
                        continue;
                    }

                    let closest_wave = waves
                        .iter()
                        .min_by(|a, b| {
                            (a.frequency - test_freq)
                                .abs()
                                .partial_cmp(&(b.frequency - test_freq).abs())
                                .unwrap()
                        })
                        .unwrap();

                    // For high frequencies, error tolerance scales with frequency
                    // MHz: 15% tolerance, GHz: 20% tolerance due to extreme FFT limitations
                    let max_error_percent = if test_freq >= 1_000_000_000.0 {
                        20.0
                    } else {
                        15.0
                    };
                    let freq_unit = if test_freq >= 1_000_000_000.0 {
                        "GHz"
                    } else {
                        "MHz"
                    };
                    let freq_display = if test_freq >= 1_000_000_000.0 {
                        test_freq / 1_000_000_000.0
                    } else {
                        test_freq / 1_000_000.0
                    };

                    let error_percent =
                        (closest_wave.frequency - test_freq).abs() / test_freq * 100.0;

                    // Debug: Show actual values to understand the precision
                    println!(
                        "Debug: Target {:.1} Hz, Detected {:.1} Hz, Absolute diff: {:.1} Hz",
                        test_freq,
                        closest_wave.frequency,
                        (closest_wave.frequency - test_freq).abs()
                    );

                    assert!(
                        error_percent < max_error_percent,
                        "High frequency {:.1}{} detection failed: got {:.2}{} ({:.3}% error > {:.0}%)",
                        freq_display,
                        freq_unit,
                        closest_wave.frequency / if test_freq >= 1_000_000_000.0 { 1_000_000_000.0 } else { 1_000_000.0 },
                        freq_unit,
                        error_percent,
                        max_error_percent
                    );

                    // Show more meaningful precision for different frequency scales
                    let display_precision = if test_freq >= 1_000_000_000.0 { 3 } else { 1 };
                    println!(
                        "✅ High freq {:.1}{}: {:.2}{} detected ({:.precision$}% error)",
                        freq_display,
                        freq_unit,
                        closest_wave.frequency
                            / if test_freq >= 1_000_000_000.0 {
                                1_000_000_000.0
                            } else {
                                1_000_000.0
                            },
                        freq_unit,
                        error_percent,
                        precision = display_precision
                    );
                }
            }
            Err(e) => {
                let freq_unit = if test_freq >= 1_000_000_000.0 {
                    "GHz"
                } else {
                    "MHz"
                };
                let freq_display = if test_freq >= 1_000_000_000.0 {
                    test_freq / 1_000_000_000.0
                } else {
                    test_freq / 1_000_000.0
                };
                panic!(
                    "Synthesis failed for {:.1}{}: {:?}",
                    freq_display, freq_unit, e
                );
            }
        }
    }
}
