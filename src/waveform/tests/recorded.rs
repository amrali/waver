use super::*;

#[test]
fn test_recorded_waveform_comprehensive() {
    // Test basic recorded waveform creation and iteration
    let samples = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let wf = Waveform::<f32>::from_recorded_samples(44100.0, &samples);

    // Verify source data is stored correctly
    if let WaveformSource::Recorded(ref data) = wf.source {
        assert_eq!(*data, samples);
    } else {
        panic!("Expected recorded waveform");
    }

    // Verify iteration works correctly
    let collected: Vec<f32> = wf.iter().collect();
    assert_eq!(collected, samples);

    // Test error conditions on recorded waveform
    let error_samples = vec![0.1f32, 0.2, 0.3, 0.4, 0.5];
    let mut recorded_waveform: Waveform<f32> =
        Waveform::from_recorded_samples(44100.0, &error_samples);

    // Test superpose on recorded waveform (should error)
    let wave = Wave {
        frequency: 440.0,
        ..Default::default()
    };
    let result = recorded_waveform.superpose(wave);
    assert!(
        result.is_err(),
        "Should not be able to superpose on recorded waveform"
    );
    assert_eq!(result.unwrap_err(), Error::UnsupportedSource);

    // Test normalize_amplitudes on recorded waveform (should error)
    let result = recorded_waveform.normalize_amplitudes();
    assert!(
        result.is_err(),
        "Should not be able to normalize recorded waveform amplitudes"
    );
    assert_eq!(result.unwrap_err(), Error::UnsupportedSource);

    // Test edge cases: empty samples
    let empty_samples: Vec<f32> = vec![];
    let empty_waveform: Waveform<f32> = Waveform::from_recorded_samples(44100.0, &empty_samples);
    let empty_result: Vec<f32> = empty_waveform.iter().collect();
    assert!(
        empty_result.is_empty(),
        "Empty waveform should produce no samples"
    );

    // Test edge cases: single sample
    let single_sample = vec![0.7f32];
    let single_waveform: Waveform<f32> = Waveform::from_recorded_samples(44100.0, &single_sample);
    let single_result: Vec<f32> = single_waveform.iter().collect();
    assert_eq!(
        single_result,
        vec![0.7f32],
        "Single sample should be preserved"
    );

    // Test edge cases: extreme sample values
    let extreme_samples = vec![-1.0f32, 1.0, 0.0, -0.5, 0.5];
    let extreme_waveform: Waveform<f32> =
        Waveform::from_recorded_samples(44100.0, &extreme_samples);
    let extreme_result: Vec<f32> = extreme_waveform.iter().collect();
    assert_eq!(
        extreme_result, extreme_samples,
        "Extreme samples should be preserved"
    );
}

#[test]
fn test_recorded_waveform_float_bit_depths() {
    // Test f32 recorded samples
    let samples_f32: Vec<f32> = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
    let wf_f32 = Waveform::<f32>::from_recorded_samples(44100.0, &samples_f32);
    let collected_f32: Vec<f32> = wf_f32.iter().collect();
    assert_eq!(
        samples_f32, collected_f32,
        "f32 recorded samples should be preserved"
    );

    // Verify source type is correct
    if let WaveformSource::Recorded(ref data) = wf_f32.source {
        assert_eq!(*data, samples_f32);
    } else {
        panic!("Expected recorded waveform for f32");
    }

    // Test f64 recorded samples
    let samples_f64: Vec<f64> = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
    let wf_f64 = Waveform::<f64>::from_recorded_samples(44100.0, &samples_f64);
    let collected_f64: Vec<f64> = wf_f64.iter().collect();
    assert_eq!(
        samples_f64, collected_f64,
        "f64 recorded samples should be preserved in storage and iteration"
    );

    // Verify source stores f64 precision
    if let WaveformSource::Recorded(ref data) = wf_f64.source {
        assert_eq!(*data, samples_f64);
    } else {
        panic!("Expected recorded waveform for f64");
    }

    // Test error conditions still work with different float bit depths
    let mut recorded_f32: Waveform<f32> = Waveform::from_recorded_samples(44100.0, &samples_f32);
    let wave = Wave {
        frequency: 440.0,
        ..Default::default()
    };

    let result = recorded_f32.superpose(wave);
    assert!(
        result.is_err(),
        "Should not be able to superpose on f32 recorded waveform"
    );
    assert_eq!(result.unwrap_err(), Error::UnsupportedSource);

    let result = recorded_f32.normalize_amplitudes();
    assert!(
        result.is_err(),
        "Should not be able to normalize f32 recorded waveform amplitudes"
    );
    assert_eq!(result.unwrap_err(), Error::UnsupportedSource);

    // Test empty samples with different bit depths
    let empty_f32: Vec<f32> = vec![];
    let empty_wf_f32: Waveform<f32> = Waveform::from_recorded_samples(44100.0, &empty_f32);
    let empty_result_f32: Vec<f32> = empty_wf_f32.iter().collect();
    assert!(
        empty_result_f32.is_empty(),
        "Empty f32 waveform should produce no samples"
    );

    // Test precision differences between f32 and f64
    let precise_samples_f64: Vec<f64> = vec![0.123456789012345, 0.987654321098765];
    let wf_precise_f64 = Waveform::<f64>::from_recorded_samples(44100.0, &precise_samples_f64);
    let collected_precise_f64: Vec<f64> = wf_precise_f64.iter().collect();
    assert_eq!(
        precise_samples_f64, collected_precise_f64,
        "f64 should preserve higher precision samples"
    );
}
