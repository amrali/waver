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

#[test]
fn test_modulation_comprehensive() {
    // Test From conversions
    let static_mod: Modulation<f32> = 0.5f32.into();
    assert_eq!(static_mod, Modulation::Static(0.5));
    let envelope_data = vec![0.0, 0.5, 1.0, 0.5, 0.0];
    let envelope_mod: Modulation<f32> = envelope_data.clone().into();
    assert_eq!(envelope_mod, Modulation::Envelope(envelope_data));
    let lfo_wave = Wave {
        sample_rate: 44100.0,
        frequency: 5.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(0.3),
        func: WaveFunc::Sine,
    };
    let lfo_mod: Modulation<f32> = lfo_wave.clone().into();
    assert_eq!(lfo_mod, Modulation::LFO(Box::new(lfo_wave)));

    // Test amplitude modulation with LFO
    let amplitude_lfo = Wave {
        sample_rate: 44100.0,
        frequency: 5.0, // 5 Hz tremolo
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(0.3),
        func: WaveFunc::Sine,
    };

    let modulated_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::LFO(Box::new(amplitude_lfo)),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = modulated_wave.iter().take(4410).collect(); // 0.1 seconds
    assert_eq!(samples.len(), 4410);
    assert!(samples.iter().all(|x| x.is_finite()));

    // Verify amplitude modulation variation
    let max_amplitude = samples.iter().fold(0.0f32, |max, &val| max.max(val.abs()));
    let min_amplitude = samples.iter().fold(f32::MAX, |min, &val| {
        if val.abs() < 0.001 {
            min
        } else {
            min.min(val.abs())
        }
    });
    assert!(
        max_amplitude > min_amplitude,
        "Amplitude modulation should create variation"
    );
    assert!(
        max_amplitude < 1.0,
        "Maximum amplitude should be reasonable"
    );

    // Test phase modulation with LFO
    let phase_lfo = Wave {
        sample_rate: 44100.0,
        frequency: 2.0, // 2 Hz vibrato
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(PI / 4.0),
        func: WaveFunc::Sine,
    };

    let phase_modulated_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::LFO(Box::new(phase_lfo)),
        amplitude: Modulation::Static(1.0),
        func: WaveFunc::Sine,
    };

    let phase_samples: Vec<f32> = phase_modulated_wave.iter().take(4410).collect();
    assert_eq!(phase_samples.len(), 4410);
    let max_sample = phase_samples
        .iter()
        .fold(0.0f32, |max, &val| max.max(val.abs()));
    assert!(
        max_sample <= 1.1,
        "Phase modulation samples should be within bounds"
    );
}

#[test]
fn test_envelope_modulation_comprehensive() {
    // Test amplitude modulation with envelope
    let envelope = vec![0.0, 0.25, 0.5, 0.75, 1.0, 0.75, 0.5, 0.25, 0.0];

    let modulated_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Envelope(envelope.clone()),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = modulated_wave.iter().take(envelope.len()).collect();
    assert_eq!(samples.len(), envelope.len());

    // First sample should be close to 0 (amplitude 0.0)
    assert!(
        samples[0].abs() < 0.001,
        "First sample should be near zero, got: {}",
        samples[0]
    );

    // Middle sample should be largest (amplitude 1.0)
    let mid_index = envelope.len() / 2;
    assert!(
        samples[mid_index].abs() > 0.1,
        "Middle sample should have significant amplitude"
    );

    // Last sample should be close to 0 again
    assert!(
        samples[envelope.len() - 1].abs() < 0.001,
        "Last sample should be near zero"
    );

    // Test phase modulation with envelope
    let phase_envelope = vec![0.0, PI / 4.0, PI / 2.0, PI / 4.0, 0.0];

    let phase_modulated_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Envelope(phase_envelope.clone()),
        amplitude: Modulation::Static(1.0),
        func: WaveFunc::Sine,
    };

    let phase_samples: Vec<f32> = phase_modulated_wave
        .iter()
        .take(phase_envelope.len())
        .collect();
    assert_eq!(phase_samples.len(), phase_envelope.len());
    assert!(
        !phase_samples.iter().all(|&x| x == phase_samples[0]),
        "Samples should vary with phase modulation"
    );

    // Test empty envelope - should use default behavior
    let empty_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Envelope(vec![]),
        amplitude: Modulation::Static(1.0),
        func: WaveFunc::Sine,
    };

    let empty_samples: Vec<f32> = empty_wave.iter().take(5).collect();
    assert_eq!(
        empty_samples.len(),
        5,
        "Empty envelope should produce samples with fallback behavior"
    );
    assert!(
        empty_samples.iter().all(|&x| x.is_finite()),
        "Empty envelope samples should be finite"
    );

    // Test single-value envelope
    let single_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Envelope(vec![0.7]),
        func: WaveFunc::Sine,
    };

    let single_samples: Vec<f32> = single_wave.iter().take(5).collect();
    assert_eq!(
        single_samples.len(),
        5,
        "Single envelope should produce all requested samples"
    );
    assert!(
        single_samples.iter().all(|&x| x.abs() <= 0.8),
        "Single envelope samples should be scaled by envelope value"
    );
    assert!(
        single_samples.iter().any(|&x| x.abs() > 0.1),
        "Single envelope should produce non-trivial samples"
    );
}

#[test]
fn test_complex_modulation_combination() {
    let amplitude_lfo = Wave {
        sample_rate: 44100.0,
        frequency: 3.0,
        phase: Modulation::Static(0.0),
        amplitude: Modulation::Static(0.2),
        func: WaveFunc::Sine,
    };

    let phase_lfo = Wave {
        sample_rate: 44100.0,
        frequency: 7.0,
        phase: Modulation::Static(PI / 2.0),
        amplitude: Modulation::Static(PI / 6.0),
        func: WaveFunc::Cosine,
    };

    let complex_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::LFO(Box::new(phase_lfo)),
        amplitude: Modulation::LFO(Box::new(amplitude_lfo)),
        func: WaveFunc::Sine,
    };

    let samples: Vec<f32> = complex_wave.iter().take(4410).collect(); // 0.1 seconds

    // Verify complex modulation produces varied output
    assert_eq!(samples.len(), 4410);
    let variance = calculate_variance(&samples);
    assert!(
        variance > 0.01,
        "Complex modulation should produce significant variation"
    );
}

#[test]
fn test_modulation_with_different_wave_functions() {
    let functions = [
        WaveFunc::Sine,
        WaveFunc::Cosine,
        WaveFunc::Square,
        WaveFunc::Sawtooth,
        WaveFunc::Triangle,
    ];

    for func in &functions {
        let lfo = Wave {
            sample_rate: 44100.0,
            frequency: 4.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(0.5),
            func: *func,
        };

        let modulated_wave = Wave {
            sample_rate: 44100.0,
            frequency: 220.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::LFO(Box::new(lfo)),
            func: WaveFunc::Sine,
        };

        let samples: Vec<f32> = modulated_wave.iter().take(100).collect();
        assert!(
            !samples.is_empty(),
            "Should generate samples for {:?} modulation",
            func
        );
        assert!(
            samples.iter().any(|&x| x.abs() > 0.001),
            "Should have non-zero samples for {:?}",
            func
        );
    }
}
