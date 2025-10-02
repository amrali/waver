//! Spectrum analysis tests including complex signals, noise handling, and harmonic content.

use super::*;

#[test]
fn test_spectrum_complex_signals() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;

    // Test 1: Signal with DC component
    let samples_dc: Vec<f32> = (0..44100)
        .map(|i| {
            let t = i as f32 / sample_rate;
            1.0 + 0.5 * (2.0 * PI * 440.0 * t).sin() // DC + AC
        })
        .collect();

    let waveform_dc: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples_dc);
    let spectrum_dc: Spectrum<f32> = spectrum(&waveform_dc, samples_dc.len());

    // Should detect both DC and fundamental frequency
    let dc_magnitude = spectrum_dc.data[0].1;
    assert!(dc_magnitude > 0.0, "Should detect DC component");

    let fundamental_bin = (440.0 / spectrum_dc.frequency_resolution).round() as usize;
    assert!(
        fundamental_bin < spectrum_dc.data.len(),
        "Fundamental frequency bin {} out of range",
        fundamental_bin
    );
    assert!(
        spectrum_dc.data[fundamental_bin].1 > dc_magnitude * 0.1,
        "Should detect AC component"
    );

    // Test 2: Signal with noise
    let frequency = 1000.0;
    let samples_noise: Vec<f32> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = (2.0 * PI * frequency * t).sin();
            let noise = 0.1 * ((i * 17 + 31) as f32).sin(); // Pseudo-random noise
            signal + noise
        })
        .collect();

    let waveform_noise: Waveform<f32> =
        Waveform::from_recorded_samples(sample_rate, &samples_noise);
    let spectrum_noise: Spectrum<f32> = spectrum(&waveform_noise, samples_noise.len());

    // Should still detect the main frequency despite noise
    let (dominant_freq, _, _) = spectrum_noise
        .data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .expect("Spectrum should have at least one frequency bin");

    assert!(
        (dominant_freq - frequency).abs() < spectrum_noise.frequency_resolution * 2.0,
        "Should detect main frequency despite noise: got {:.1}Hz, expected {:.1}Hz",
        dominant_freq,
        frequency
    );

    // Test 3: Complex harmonic content
    let fundamental = 220.0;
    let samples_harmonic: Vec<f32> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = (2.0 * PI * fundamental * t).sin() +
                       0.5 * (2.0 * PI * fundamental * 2.0 * t).sin() + // 2nd harmonic
                       0.25 * (2.0 * PI * fundamental * 3.0 * t).sin() + // 3rd harmonic
                       0.125 * (2.0 * PI * fundamental * 4.0 * t).sin(); // 4th harmonic
            signal * 0.3
        })
        .collect();

    let waveform_harmonic: Waveform<f32> =
        Waveform::from_recorded_samples(sample_rate, &samples_harmonic);
    let spectrum_harmonic: Spectrum<f32> = spectrum(&waveform_harmonic, samples_harmonic.len());

    // Should detect fundamental and harmonics
    let mut detected_peaks = 0;
    for expected_freq in &[
        fundamental,
        fundamental * 2.0,
        fundamental * 3.0,
        fundamental * 4.0,
    ] {
        let bin = (*expected_freq / spectrum_harmonic.frequency_resolution).round() as usize;
        assert!(
            bin < spectrum_harmonic.data.len(),
            "Frequency bin {} out of range for frequency {:.1}Hz",
            bin,
            expected_freq
        );
        if spectrum_harmonic.data[bin].1 > 0.1 {
            detected_peaks += 1;
        }
    }

    assert!(
        detected_peaks >= 3,
        "Should detect at least 3 of 4 harmonic peaks, found {}",
        detected_peaks
    );
}

#[test]
fn test_spectrum_with_noise() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;
    let frequency = 1000.0;

    // Create signal with added noise
    let samples: Vec<f32> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = (2.0 * PI * frequency * t).sin();
            let noise = 0.1 * ((i * 17 + 31) as f32).sin(); // Pseudo-random noise
            let combined = signal + noise;
            combined
        })
        .collect();

    let waveform: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples);
    let spectrum_result: Spectrum<f32> = spectrum(&waveform, samples.len());

    // Should still detect the main frequency despite noise
    let (dominant_freq, _, _) = spectrum_result
        .data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .expect("Spectrum should have at least one frequency bin");

    assert!((dominant_freq - frequency).abs() < spectrum_result.frequency_resolution * 2.0);
}

#[test]
fn test_complex_harmonic_content() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;
    let fundamental = 220.0;

    // Create signal with multiple harmonics
    let samples: Vec<f32> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = (2.0 * PI * fundamental * t).sin() +
                       0.5 * (2.0 * PI * fundamental * 2.0 * t).sin() + // 2nd harmonic
                       0.25 * (2.0 * PI * fundamental * 3.0 * t).sin() + // 3rd harmonic
                       0.125 * (2.0 * PI * fundamental * 4.0 * t).sin(); // 4th harmonic
            signal * 0.3
        })
        .collect();

    let waveform: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples);
    let spectrum_result: Spectrum<f32> = spectrum(&waveform, samples.len());

    // Should detect fundamental and harmonics
    let mut detected_peaks = 0;
    for expected_freq in &[
        fundamental,
        fundamental * 2.0,
        fundamental * 3.0,
        fundamental * 4.0,
    ] {
        let bin = (*expected_freq / spectrum_result.frequency_resolution).round() as usize;
        assert!(
            bin < spectrum_result.data.len(),
            "Frequency bin {} out of range for frequency {:.1}Hz",
            bin,
            expected_freq
        );
        if spectrum_result.data[bin].1 > 0.1 {
            detected_peaks += 1;
        }
    }

    assert!(
        detected_peaks >= 3,
        "Should detect at least 3 of 4 harmonic peaks"
    );
}

#[test]
fn test_time_spectrum_consistency() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let samples: Vec<f32> = (0..sample_rate as u32)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let signal = (2.0 * PI * frequency * t).sin();
            signal * 0.7
        })
        .collect();

    let waveform: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples);
    let spectrogram = time_spectrum(&waveform, 1024, 512).unwrap();

    // All spectra in the spectrogram should have similar dominant frequency
    let mut consistent_detections = 0;
    for spectrum in &spectrogram {
        let (dominant_freq, _, _) = spectrum
            .data
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();

        if (dominant_freq - frequency).abs() < spectrum.frequency_resolution * 1.5 {
            consistent_detections += 1;
        }
    }

    let consistency_ratio = consistent_detections as f32 / spectrogram.len() as f32;
    assert!(
        consistency_ratio > 0.90,
        "Should have highly consistent frequency detection across time: {:.1}% (expected > 90%)",
        consistency_ratio * 100.0
    );
}

#[test]
fn test_stft_edge_cases() {
    let sample_rate = 44100.0;
    let samples: Vec<f32> = vec![0.0; 100]; // Very short signal
    let waveform: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples);

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
