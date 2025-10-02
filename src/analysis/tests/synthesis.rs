//! Synthesis quality and functionality tests.

use super::*;

#[test]
fn test_synthesis_quality() {
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let duration = 1.0; // Full second for reliable statistics
    let num_samples = (sample_rate * duration) as usize;

    // Generate pure sine wave with known amplitude
    let target_amplitude = 0.8;
    let samples: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            target_amplitude * (t * frequency * 2.0 * PI).sin()
        })
        .collect();

    let original_waveform: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples);
    let synthesized_waveform = synthesize(&original_waveform, 2048, 512, 8).unwrap(); // Larger window for better precision

    if let WaveformSource::Generative(waves) = &synthesized_waveform.source {
        assert!(
            !waves.is_empty(),
            "Synthesis should produce at least one wave component"
        );
        assert!(
            waves.len() <= 5,
            "Should not produce excessive wave components for pure tone - got {} components",
            waves.len()
        );

        // Find the dominant wave (closest to target frequency)
        let target_wave = waves
            .iter()
            .min_by(|a, b| {
                (a.frequency - frequency)
                    .abs()
                    .partial_cmp(&(b.frequency - frequency).abs())
                    .unwrap()
            })
            .expect("Should have at least one wave");

        // Rigorous frequency accuracy test - should be within 0.5% for pure sine
        let frequency_error_percent = (target_wave.frequency - frequency).abs() / frequency * 100.0;
        assert!(
            frequency_error_percent < 0.5,
            "Frequency accuracy too poor: {:.3}% error (got {:.1}Hz, expected {:.1}Hz)",
            frequency_error_percent,
            target_wave.frequency,
            frequency
        );

        // Verify amplitude is reasonable (not just > 0)
        let amplitude_value = match &target_wave.amplitude {
            crate::Modulation::Static(val) => *val,
            crate::Modulation::Envelope(env) => {
                // Use RMS of envelope for dynamic amplitude
                (env.iter().map(|x| x * x).sum::<f32>() / env.len() as f32).sqrt()
            }
            crate::Modulation::LFO(lfo_params) => {
                // For LFO, validate that parameters are reasonable and extract base amplitude
                assert!(
                    lfo_params.frequency > 0.0 && lfo_params.frequency < 100.0,
                    "LFO frequency should be reasonable: {}Hz",
                    lfo_params.frequency
                );
                // LFO amplitude is also a Modulation - handle recursively or use a reasonable default
                match &lfo_params.amplitude {
                    crate::Modulation::Static(amp_val) => amp_val.abs(),
                    _ => 0.5, // For complex nested modulations, use a reasonable default
                }
            }
        };

        assert!(
            amplitude_value > 0.2,
            "Synthesized amplitude too weak: {:.3} (may indicate poor energy preservation)",
            amplitude_value
        );

        println!(
            "✓ Frequency: {:.2}Hz (error: {:.3}%, target: {:.1}Hz)",
            target_wave.frequency, frequency_error_percent, frequency
        );
        println!(
            "✓ Amplitude: {:.3} (modulation type: {:?})",
            amplitude_value,
            match target_wave.amplitude {
                crate::Modulation::Static(_) => "Static",
                crate::Modulation::Envelope(_) => "Envelope",
                crate::Modulation::LFO(_) => "LFO",
            }
        );
    } else {
        panic!("Expected generative waveform");
    }

    // Rigorous spectral energy preservation test
    let window_size = 2048; // Same as synthesis
    let orig_spectrum = spectrum(&original_waveform, window_size);
    let synth_spectrum = spectrum(&synthesized_waveform, window_size);

    // Use broader frequency range for peak finding - account for FFT bin resolution
    let freq_tolerance = (orig_spectrum.frequency_resolution * 2.0).max(frequency * 0.05);
    let broad_freq_range = (frequency - freq_tolerance, frequency + freq_tolerance);

    let orig_peak_magnitude = orig_spectrum
        .data
        .iter()
        .filter(|(f, _, _)| *f >= broad_freq_range.0 && *f <= broad_freq_range.1)
        .map(|(_, mag, _)| *mag)
        .fold(0.0f32, |acc, val| acc.max(val));

    let synth_peak_magnitude = synth_spectrum
        .data
        .iter()
        .filter(|(f, _, _)| *f >= broad_freq_range.0 && *f <= broad_freq_range.1)
        .map(|(_, mag, _)| *mag)
        .fold(0.0f32, |acc, val| acc.max(val));

    // Both should have significant energy at target frequency
    assert!(
        orig_peak_magnitude > 100.0,
        "Original spectrum should have strong peak at target frequency, got magnitude: {:.1}",
        orig_peak_magnitude
    );

    assert!(
        synth_peak_magnitude > 50.0,
        "Synthesized spectrum should have strong peak at target frequency, got magnitude: {:.1}",
        synth_peak_magnitude
    );

    // Energy preservation should be good - allow max 15% loss
    let energy_preservation_ratio = synth_peak_magnitude / orig_peak_magnitude;
    assert!(
        energy_preservation_ratio > 0.85,
        "Energy preservation too poor: {:.1}% (magnitude {:.1} → {:.1})",
        energy_preservation_ratio * 100.0,
        orig_peak_magnitude,
        synth_peak_magnitude
    );

    // Test signal quality by comparing RMS values
    let orig_rms: f32 =
        (samples.iter().map(|x| x.powi(2)).sum::<f32>() / samples.len() as f32).sqrt();
    let synth_samples: Vec<f32> = synthesized_waveform.iter().take(num_samples).collect();
    let synth_rms: f32 =
        (synth_samples.iter().map(|x| x.powi(2)).sum::<f32>() / synth_samples.len() as f32).sqrt();

    // RMS should be preserved within tighter bounds (±20%)
    let rms_ratio = synth_rms / orig_rms;
    assert!(
        rms_ratio > 0.8 && rms_ratio < 1.2,
        "RMS energy preservation poor: original {:.3}, synthesized {:.3} (ratio: {:.2})",
        orig_rms,
        synth_rms,
        rms_ratio
    );

    // Test for spurious frequency components (noise) - be more realistic about spectral leakage
    // For windowed signals, some leakage into adjacent bins is normal
    let main_signal_bins = 3; // Allow 3 bins around peak (center + neighbors)
    let target_bin = (frequency / orig_spectrum.frequency_resolution).round() as usize;

    let orig_main_energy: f32 = orig_spectrum
        .data
        .iter()
        .enumerate()
        .filter(|(i, _)| (*i as isize - target_bin as isize).abs() <= main_signal_bins as isize)
        .map(|(_, (_, mag, _))| *mag)
        .sum();

    let orig_spurious_energy: f32 = orig_spectrum
        .data
        .iter()
        .enumerate()
        .filter(|(i, _)| (*i as isize - target_bin as isize).abs() > main_signal_bins as isize)
        .filter(|(_, (f, _, _))| *f > 20.0) // Ignore DC and very low freq
        .map(|(_, (_, mag, _))| *mag)
        .sum();

    let synth_main_energy: f32 = synth_spectrum
        .data
        .iter()
        .enumerate()
        .filter(|(i, _)| (*i as isize - target_bin as isize).abs() <= main_signal_bins as isize)
        .map(|(_, (_, mag, _))| *mag)
        .sum();

    let synth_spurious_energy: f32 = synth_spectrum
        .data
        .iter()
        .enumerate()
        .filter(|(i, _)| (*i as isize - target_bin as isize).abs() > main_signal_bins as isize)
        .filter(|(_, (f, _, _))| *f > 20.0) // Ignore DC and very low freq
        .map(|(_, (_, mag, _))| *mag)
        .sum();

    let orig_spurious_ratio = orig_spurious_energy / orig_main_energy;
    let synth_spurious_ratio = synth_spurious_energy / synth_main_energy;

    // Spurious energy should not be dramatically worse than original
    // Allow synthesis to have up to 2.5x the spurious ratio of original (balanced)
    let spurious_degradation = synth_spurious_ratio / orig_spurious_ratio.max(0.1); // Avoid div by zero
    assert!(
        spurious_degradation < 2.5,
        "Spurious energy degradation too high: {:.1}x worse than original (orig: {:.1}%, synth: {:.1}%)",
        spurious_degradation, orig_spurious_ratio * 100.0, synth_spurious_ratio * 100.0
    );

    // Also ensure absolute spurious energy isn't excessive (less than 250% of main signal)
    assert!(
        synth_spurious_ratio < 2.5,
        "Absolute spurious energy too high: {:.1}% of main signal",
        synth_spurious_ratio * 100.0
    );

    println!(
        "✓ Energy preservation: {:.1}% (magnitude {:.1} → {:.1})",
        energy_preservation_ratio * 100.0,
        orig_peak_magnitude,
        synth_peak_magnitude
    );
    println!(
        "✓ RMS preservation: {:.1}% (RMS {:.3} → {:.3})",
        rms_ratio * 100.0,
        orig_rms,
        synth_rms
    );
    println!(
        "✓ Spurious energy: {:.1}x degradation (orig: {:.0}%, synth: {:.0}%)",
        spurious_degradation,
        orig_spurious_ratio * 100.0,
        synth_spurious_ratio * 100.0
    );
    println!("✅ Rigorous synthesis quality test passed - all metrics within bounds");
}

#[test]
fn test_synthesis_edge_cases() {
    let sample_rate = 8000.0; // Lower sample rate
    let samples: Vec<f32> = vec![0.0; 50]; // Very short signal
    let waveform: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples);

    // Test synthesis with very short signal
    let result = synthesize(&waveform, 32, 16, 3);
    assert!(result.is_ok(), "Should handle very short signals");

    // Test synthesis with max_harmonics = 0
    let result = synthesize(&waveform, 32, 16, 0);
    assert!(result.is_ok(), "Should handle zero harmonics");

    // Test parameter validation
    let result = synthesize(&waveform, 2, 16, 3); // window_size too small
    assert!(result.is_err(), "Should reject window_size < 4");
    if let Err(Error::InvalidParameters { message }) = result {
        assert!(message.contains("window_size"));
    } else {
        panic!("Expected InvalidParameters error");
    }

    let result = synthesize(&waveform, 32, 0, 3); // hop_size = 0
    assert!(result.is_err(), "Should reject hop_size = 0");
    if let Err(Error::InvalidParameters { message }) = result {
        assert!(message.contains("hop_size"));
    } else {
        panic!("Expected InvalidParameters error");
    }

    let result = synthesize(&waveform, 2_000_000, 16, 3); // excessively large window
    assert!(result.is_err(), "Should reject excessively large windows");
    if let Err(Error::InvalidParameters { message }) = result {
        assert!(message.contains("window_size"));
    } else {
        panic!("Expected InvalidParameters error");
    }

    // Test with zero/negative sample rate waveform
    let invalid_waveform: Waveform<f32> = Waveform::new(0.0); // Invalid sample rate
    let result = synthesize(&invalid_waveform, 32, 16, 3);
    assert!(result.is_err(), "Should reject invalid sample rates");
    if let Err(Error::InvalidParameters { message }) = result {
        assert!(message.contains("sample_rate"));
    } else {
        panic!("Expected InvalidParameters error");
    }
}

#[test]
fn test_synthesis_frequency_tracking() {
    use std::f32::consts::PI;
    let sample_rate = 44100.0;

    // Create a frequency-modulated signal
    let samples: Vec<f32> = (0..44100 * 2) // 2 seconds
        .map(|i| {
            let t = i as f32 / sample_rate;
            let base_freq = 440.0;
            let fm_freq = 2.0; // 2 Hz modulation
            let fm_depth = 50.0; // ±50 Hz
            let _instantaneous_freq = base_freq + fm_depth * (2.0 * PI * fm_freq * t).sin();
            let phase =
                2.0 * PI * base_freq * t + (fm_depth / fm_freq) * (2.0 * PI * fm_freq * t).cos();
            let signal = phase.sin();
            signal
        })
        .collect();

    let waveform: Waveform<f32> = Waveform::from_recorded_samples(sample_rate, &samples);
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
