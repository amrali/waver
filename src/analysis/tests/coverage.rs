//! Frequency range coverage and comprehensive analysis tests.

use super::*;

#[test]
fn test_frequency_range_coverage() {
    // Comprehensive test of frequency range coverage including extreme cases
    let sample_rate = 44100.0;

    // Test cases: (frequency, expected_error_percent, description)
    let test_cases = vec![
        // Standard audio range - pushing for maximum tightness
        (50.0, 2.2, "low frequency"), // Based on actual 2.18% performance
        (440.0, 0.5, "audio frequency"), // Based on actual 0.45% performance
        (2000.0, 0.08, "mid frequency"), // Based on actual 0.07% performance
        (8000.0, 0.04, "high frequency"), // Based on actual 0.03% performance
        // Extreme low frequencies (with appropriate sample rates)
        (0.5, 0.5, "sub-Hz frequency"), // Based on actual 0.48% performance
        (1.0, 0.5, "very low frequency"), // Based on actual 0.48% performance
        // Extreme range within Nyquist
        (1.0, 0.5, "very low Hz"), // Based on actual 0.48% performance
        (sample_rate * 0.45, 0.05, "near Nyquist"), // ABSURDLY aggressive from 0.1%
        // Realistic high frequencies that should work well
        (15000.0, 0.05, "15 kHz frequency"), // INSANELY aggressive from 0.1%
        (18000.0, 0.05, "18 kHz frequency"), // INSANELY aggressive from 0.1%
        // MHz/GHz frequencies - pushing these too
        (1_000_000.0, 0.2, "1 MHz frequency"), // INSANELY aggressive from 0.4%
        (5_000_000.0, 0.4, "5 MHz frequency"), // INSANELY aggressive from 0.75%
        (1_000_000_000.0, 0.5, "1 GHz frequency"), // INSANELY aggressive from 1.0%
    ];

    for &(test_freq, max_error_percent, description) in &test_cases {
        // Adjust parameters for extreme cases
        let (adjusted_sample_rate, window_size, duration) = if test_freq < 2.0 {
            // Sub-Hz requires special handling
            let sr = if test_freq < 1.0 { 100.0 } else { 200.0 };
            let ws = 2048;
            let dur = (25.0f32 / test_freq).max(ws as f32 * 4.0 / sr); // Ensure enough cycles
            (sr, ws, dur)
        } else if test_freq >= 1_000_000_000.0 {
            // GHz range - needs very high sample rate
            let sr = test_freq * 10.0; // 10x oversampling
            let ws = 2048;
            let dur = (ws as f32 * 4.0) / sr; // Ensure enough samples for window
            (sr, ws, dur)
        } else if test_freq >= 1_000_000.0 {
            // MHz range - high sample rate needed
            let sr = test_freq * 50.0; // 50x oversampling for MHz
            let ws = 2048;
            let dur = (ws as f32 * 4.0) / sr; // Ensure enough samples for window
            (sr, ws, dur)
        } else {
            // Standard case - all other frequencies use normal parameters
            (sample_rate, 1024, 1.0)
        };

        let num_samples = (adjusted_sample_rate * duration) as usize;
        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / adjusted_sample_rate;
                (t * test_freq * 2.0 * PI).sin()
            })
            .collect();

        let waveform: Waveform<f32> =
            Waveform::from_recorded_samples(adjusted_sample_rate, &samples);

        let synthesized = synthesize(&waveform, window_size, window_size / 4, 5).expect(&format!(
            "Synthesis should not fail for {description} {test_freq}Hz"
        ));

        if let WaveformSource::Generative(waves) = &synthesized.source {
            assert!(
                !waves.is_empty(),
                "No waves detected for {description} {test_freq}Hz"
            );

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

            let actual_error = (closest_freq - test_freq).abs() / test_freq * 100.0;
            assert!(
                actual_error < max_error_percent,
                "{description} {test_freq}Hz synthesis failed: {actual_error:.2}% error (expected < {max_error_percent:.1}%)"
            );
        }
    }
}
