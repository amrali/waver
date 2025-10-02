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
fn test_wave_default() {
    let wave: Wave<f32> = Default::default();
    assert_eq!(
        wave,
        Wave {
            sample_rate: 0.0,
            frequency: 0.0,
            phase: Modulation::Static(0.0),
            amplitude: Modulation::Static(1.0),
            func: WaveFunc::Sine
        }
    );
}

#[test]
fn test_wave_function_iterations() {
    // Test all wave functions with expected outputs
    let test_cases = [
        (
            WaveFunc::Sine,
            [0.0, 0.9980267, -0.1253336, -0.98228717, 0.24869062],
        ),
        (
            WaveFunc::Cosine,
            [1.0, -0.06279071, -0.99211466, 0.18738186, 0.968583],
        ),
        (WaveFunc::Square, [1.0, 1.0, -1.0, -1.0, 1.0]),
        (
            WaveFunc::Sawtooth,
            [-1.0, -0.47999996, 0.04000008, 0.5600002, -0.91999984],
        ),
        (
            WaveFunc::Triangle,
            [0.0, 0.96000004, -0.08000016, -0.8799999, 0.16000032],
        ),
    ];

    for (func, expected) in test_cases.iter() {
        let wave = Wave {
            sample_rate: 500.0,
            frequency: 130.0,
            func: *func,
            ..Default::default()
        };
        let res: Vec<f32> = wave.iter().take(5).collect();

        for (i, (&actual, &expected)) in res.iter().zip(expected.iter()).enumerate() {
            if expected == 1.0 || expected == -1.0 || expected == 0.0 {
                // Exact comparison for simple values
                assert_eq!(
                    actual, expected,
                    "Wave function {:?} sample {} mismatch",
                    func, i
                );
            } else {
                // Tolerance comparison for computed values
                assert!(
                    (actual - expected).abs() < 1e-6,
                    "Wave function {:?} sample {} mismatch: expected {}, got {}",
                    func,
                    i,
                    expected,
                    actual
                );
            }
        }
    }
}

#[test]
fn test_wave_phase_shift() {
    let wave = Wave {
        sample_rate: 500.0,
        frequency: 120.0,
        phase: (PI / 2.0).into(),
        ..Default::default()
    };
    let res: Vec<f32> = wave.iter().take(5).collect();
    assert_eq!(res[0], 1.0);
}

#[test]
fn test_wave_formatting() {
    // Test Wave display formatting
    let wave = Wave {
        sample_rate: 500.0,
        frequency: 120.0,
        phase: Modulation::Static(PI / 2.0),
        ..Default::default()
    };
    let fmt_string = format!("{wave}");
    assert_eq!(
        fmt_string,
        "<Func: Sine, Freq: 120Hz, Ampl: 1, Sampling Freq: 500Hz>"
    );

    // Test WaveFunc formatting
    assert_eq!(format!("{}", WaveFunc::Sine), "Sine");
    assert_eq!(format!("{}", WaveFunc::Cosine), "Cosine");
    assert_eq!(format!("{}", WaveFunc::Square), "Square");
    assert_eq!(format!("{}", WaveFunc::Sawtooth), "Sawtooth");
    assert_eq!(format!("{}", WaveFunc::Triangle), "Triangle");

    // Test complex wave display formatting
    let complex_wave = Wave {
        sample_rate: 44100.0,
        frequency: 440.0,
        phase: Modulation::Static(PI / 4.0),
        amplitude: Modulation::Static(0.8),
        func: WaveFunc::Triangle,
    };

    let display_str = format!("{}", complex_wave);
    assert!(
        display_str.contains("Triangle"),
        "Should display wave function"
    );
    assert!(display_str.contains("440Hz"), "Should display frequency");
    assert!(
        display_str.contains("44100Hz"),
        "Should display sample rate"
    );
}
