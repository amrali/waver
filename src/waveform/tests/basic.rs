use super::*;

#[test]
fn test_waveform_basic_functionality() {
    // Test single wave match behavior
    let w3khz = Wave {
        sample_rate: 44100.0,
        frequency: 3000.0,
        ..Default::default()
    };
    let wf = Waveform::<f32>::with_wave(44100.0, w3khz.clone());

    let w1: Vec<f32> = wf.iter().take(100).collect();
    let w2: Vec<f32> = w3khz.iter().take(100).collect();

    assert_eq!(w1, w2);

    // Test empty waveform
    let wf_empty = Waveform::<f32>::new(44100.0);
    let v: Vec<f32> = wf_empty.iter().take(10).collect();
    assert_eq!(v, [0.0f32; 10]);

    // Test basic iteration
    let wf_iter = Waveform::<f32>::new(44100.0);
    let mut itr = wf_iter.into_iter();
    assert_eq!(itr.next().unwrap(), 0.0);

    // Test construction equivalence
    let wf1 = Waveform::<f32>::with_wave(
        44100.0,
        Wave {
            frequency: 3400.0,
            amplitude: 1.0.into(),
            ..Default::default()
        },
    );
    let mut wf2 = Waveform::<f32>::new(44100.0);

    let v1: Vec<f32> = wf1.iter().take(100).collect();
    wf2.superpose(Wave {
        frequency: 3400.0,
        ..Default::default()
    })
    .unwrap();
    let v2: Vec<f32> = wf2.iter().take(100).collect();

    assert_eq!(v1, v2);
}

#[test]
fn test_waveform_amplitude_normalization_comprehensive() {
    // Test basic amplitude normalization
    let mut wf = Waveform::<f32>::with_wave(
        44100.0,
        Wave {
            frequency: 4000.0,
            amplitude: 1.5.into(),
            ..Default::default()
        },
    );
    wf.superpose(Wave {
        frequency: 5000.0,
        amplitude: 0.5.into(),
        ..Default::default()
    })
    .unwrap()
    .normalize_amplitudes()
    .unwrap();

    // Check that normalization results in equal amplitudes
    if let WaveformSource::Generative(components) = wf.source {
        components
            .iter()
            .for_each(|c| assert_eq!(c.amplitude, Modulation::Static(0.5)));
    } else {
        panic!("Expected generative waveform");
    }

    // Test single wave normalization edge case
    let mut single_wave_wf = Waveform::<f32>::with_wave(
        44100.0,
        Wave {
            frequency: 440.0,
            amplitude: Modulation::Static(0.7),
            ..Default::default()
        },
    );

    let result = single_wave_wf.normalize_amplitudes();
    assert!(result.is_ok(), "Should normalize single wave");
    if let Ok(wf) = result {
        if let crate::waveform::WaveformSource::Generative(waves) = &wf.source {
            assert_eq!(
                waves[0].amplitude,
                Modulation::Static(1.0),
                "Single wave should be normalized to 1.0"
            );
        }
    }

    // Test many waves normalization edge case
    let mut many_waves_wf = Waveform::<f32>::new(44100.0);
    for i in 1..=10 {
        let wave = Wave {
            frequency: 100.0 * i as f32,
            amplitude: Modulation::Static(0.5),
            ..Default::default()
        };
        many_waves_wf.superpose(wave).unwrap();
    }

    let result = many_waves_wf.normalize_amplitudes();
    assert!(result.is_ok(), "Should normalize many waves");
    if let Ok(wf) = result {
        if let crate::waveform::WaveformSource::Generative(waves) = &wf.source {
            for wave in waves {
                assert_eq!(
                    wave.amplitude,
                    Modulation::Static(0.1),
                    "Each wave should get 1/10 amplitude"
                );
            }
        }
    }
}

#[test]
fn test_waveform_float_bit_depth_comprehensive() {
    let sample_rate = 44100.0;
    let wave = Wave {
        frequency: 440.0,
        amplitude: Modulation::Static(0.8),
        ..Default::default()
    };

    // Test f32 bit depth
    let wf_f32 = Waveform::<f32>::with_wave(sample_rate, wave.clone());
    let samples_f32: Vec<f32> = wf_f32.iter().take(10).collect();
    assert_eq!(samples_f32.len(), 10);
    assert!(
        samples_f32.iter().all(|&x| x.is_finite()),
        "f32 samples should be finite"
    );

    // Test f64 bit depth
    let wave_f64 = Wave {
        frequency: 440.0,
        amplitude: Modulation::Static(0.8),
        ..Default::default()
    };
    let wf_f64 = Waveform::<f64>::with_wave(sample_rate as f64, wave_f64);
    let samples_f64: Vec<f64> = wf_f64.iter().take(10).collect();
    assert_eq!(samples_f64.len(), 10);
    assert!(
        samples_f64.iter().all(|&x| x.is_finite()),
        "f64 samples should be finite"
    );

    // Test corner cases with specific bit depths
    let corner_wave = Wave {
        sample_rate: 44100.0,
        frequency: 1000.0,
        amplitude: Modulation::Static(1.0),
        ..Default::default()
    };

    // Test f32 corner case
    let wf_f32_corner = Waveform::<f32>::with_wave(44100.0, corner_wave.clone());
    let samples_f32_corner: Vec<f32> = wf_f32_corner.iter().take(10).collect();
    assert_eq!(
        samples_f32_corner.len(),
        10,
        "Should generate exactly 10 f32 samples"
    );
    assert!(
        samples_f32_corner.iter().any(|&x| x.abs() > 0.1),
        "Should have some significant f32 values"
    );

    // Test f64 corner case with double precision
    let corner_wave_f64 = Wave {
        sample_rate: 44100.0,
        frequency: 1000.0,
        amplitude: Modulation::Static(1.0),
        ..Default::default()
    };
    let wf_f64_corner = Waveform::<f64>::with_wave(44100.0, corner_wave_f64);
    let samples_f64_corner: Vec<f64> = wf_f64_corner.iter().take(10).collect();
    assert_eq!(
        samples_f64_corner.len(),
        10,
        "Should generate exactly 10 f64 samples"
    );
    assert!(
        samples_f64_corner.iter().any(|&x| x.abs() > 0.1),
        "f64 should produce significant samples"
    );
}

#[test]
fn test_waveform_superposition_and_stability_comprehensive() {
    use core::f32::consts::PI;

    // Test superposition with different wave functions
    let mut waveform = Waveform::<f32>::new(44100.0);

    let functions = [
        WaveFunc::Sine,
        WaveFunc::Cosine,
        WaveFunc::Square,
        WaveFunc::Sawtooth,
        WaveFunc::Triangle,
    ];

    for (i, &func) in functions.iter().enumerate() {
        let wave = Wave {
            frequency: 220.0 * (i + 1) as f32,
            amplitude: Modulation::Static(0.2),
            func,
            ..Default::default()
        };
        waveform.superpose(wave).unwrap();
    }

    let samples: Vec<f32> = waveform.iter().take(1000).collect();
    assert_eq!(samples.len(), 1000);

    // Should produce complex waveform with all functions combined
    let variance = calculate_variance(&samples);
    assert!(
        variance > 0.01,
        "Combined waveform should have significant variance"
    );

    // Test numerical stability with potential overflow conditions
    let mut stable_wf = Waveform::<f32>::new(44100.0);

    // Add waves with combined amplitude that's manageable for floats
    for i in 1..=5 {
        let wave = Wave {
            sample_rate: 44100.0,
            frequency: 440.0 * i as f32,
            amplitude: Modulation::Static(0.15), // Total: 75% (safe)
            phase: Modulation::Static(0.0),
            func: crate::WaveFunc::Sine,
        };
        stable_wf.superpose(wave).unwrap();
    }

    // Should produce samples with good stability
    let stability_samples: Vec<f32> = stable_wf.iter().take(100).collect();

    // Should get all requested samples with float precision
    assert_eq!(
        stability_samples.len(),
        100,
        "Should produce exactly 100 samples with float configuration"
    );

    // Check for significant amplitude with 5 superposed waves
    let max_amplitude = stability_samples
        .iter()
        .map(|&x| x.abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);
    assert!(
        max_amplitude > 0.1,
        "Should have significant amplitude with 5 superposed waves"
    );

    // Test modulation interaction between amplitude and phase
    let amp_envelope = vec![0.0, 0.5, 1.0, 0.5, 0.0];
    let phase_envelope = vec![0.0, PI / 4.0, PI / 2.0, PI / 4.0, 0.0];

    let modulated_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        amplitude: Modulation::Envelope(amp_envelope.clone()),
        phase: Modulation::Envelope(phase_envelope.clone()),
        func: WaveFunc::Sine,
    };

    let modulated_waveform = Waveform::<f32>::with_wave(44100.0, modulated_wave);
    let modulated_samples: Vec<f32> = modulated_waveform.iter().take(amp_envelope.len()).collect();

    assert_eq!(modulated_samples.len(), amp_envelope.len());

    // First and last samples should be near zero due to amplitude envelope
    assert!(
        modulated_samples[0].abs() < 0.01,
        "First sample should be small"
    );
    assert!(
        modulated_samples[modulated_samples.len() - 1].abs() < 0.01,
        "Last sample should be small"
    );
}
