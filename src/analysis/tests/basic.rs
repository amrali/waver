//! Basic functionality tests for spectrum analysis, time spectrum, and synthesis.

use super::*;

#[test]
fn test_spectrum_basic() {
    // Test basic spectrum analysis on recorded sine wave samples
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let samples: Vec<f32> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (t * frequency * 2.0 * PI).sin()
        })
        .collect();

    let waveform: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples);
    let spectrum_result: Spectrum<f32> = spectrum(&waveform, sample_rate as usize);

    // Verify dominant frequency detection
    let (dominant_freq, _, _) = spectrum_result
        .data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();

    assert!(
        (dominant_freq - frequency).abs() < spectrum_result.frequency_resolution,
        "Expected frequency {:.1}Hz, got {:.1}Hz (resolution: {:.2}Hz)",
        frequency,
        dominant_freq,
        spectrum_result.frequency_resolution
    );
}

#[test]
fn test_time_spectrum() {
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let samples: Vec<f32> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let sample = (t * frequency * 2.0 * PI).sin();
            sample
        })
        .collect();

    let waveform: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples);
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

        if (dominant_freq - frequency).abs() < spectrum.frequency_resolution * 1.5 {
            accurate_detections += 1;
        }
    }

    // At least 95% of frames should accurately detect the frequency for pure sine
    let accuracy_ratio = accurate_detections as f32 / spectrogram.len() as f32;
    assert!(
        accuracy_ratio > 0.95,
        "Frequency detection accuracy too low: {:.1}% (expected > 95%)",
        accuracy_ratio * 100.0
    );
}

#[test]
fn test_synthesize() {
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let samples: Vec<f32> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let sample = (t * frequency * 2.0 * PI).sin();
            sample
        })
        .collect();

    let waveform: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples);
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

        // Frequency accuracy should be better than 1% for clean sine waves
        let frequency_error = (closest_wave.frequency - frequency).abs() / frequency;
        assert!(
            frequency_error < 0.01,
            "Frequency error too large: {:.1}% (expected < 1%)",
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
