use super::*;

#[test]
fn test_waveform_edge_cases_comprehensive() {
    // Test with zero amplitude
    let zero_amp_wave = Wave {
        frequency: 440.0,
        amplitude: Modulation::Static(0.0),
        ..Default::default()
    };

    let zero_amp_waveform = Waveform::<f32>::with_wave(44100.0, zero_amp_wave);
    let zero_samples: Vec<f32> = zero_amp_waveform.iter().take(100).collect();
    assert!(
        zero_samples.iter().all(|&x| x == 0.0),
        "Zero amplitude should produce zero samples"
    );

    // Test with zero frequency
    let zero_freq_wave = Wave {
        frequency: 0.0,
        amplitude: Modulation::Static(0.5),
        ..Default::default()
    };

    let zero_freq_waveform = Waveform::<f32>::with_wave(44100.0, zero_freq_wave);
    let freq_samples: Vec<f32> = zero_freq_waveform.iter().take(100).collect();
    let first_sample = freq_samples[0];
    assert!(
        freq_samples
            .iter()
            .all(|&x| (x - first_sample).abs() < 1e-6),
        "Zero frequency should produce constant output"
    );

    // Test with extreme sample rates
    let _low_sr_waveform = Waveform::<f32>::new(8.0);
    let _high_sr_waveform = Waveform::<f32>::new(192000.0);

    // Test iterator stability with floating point precision
    let mut stable_wf = Waveform::<f32>::with_wave(
        44100.0,
        Wave {
            frequency: 4000.0,
            amplitude: 1.0.into(),
            ..Default::default()
        },
    );
    stable_wf
        .superpose(Wave {
            frequency: 5000.0,
            amplitude: 0.5.into(),
            ..Default::default()
        })
        .unwrap();
    let stable_samples: Vec<f32> = stable_wf.iter().take(100).collect();
    assert_eq!(
        stable_samples.len(),
        100,
        "Float iterator should be stable and produce all requested samples"
    );
    assert!(
        stable_samples.iter().all(|&x| x.is_finite()),
        "All float samples should be finite"
    );
}
