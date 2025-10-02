# Waver [![CI](https://github.com/amrali/waver/actions/workflows/main.yml/badge.svg)](https://github.com/amrali/waver/actions/workflows/main.yml) [![codecov](https://codecov.io/gh/amrali/waver/branch/master/graph/badge.svg?token=fN3pEuLaAB)](https://codecov.io/gh/amrali/waver) [![Crates.io](https://img.shields.io/crates/v/waver.svg?logo=rust)](https://crates.io/crates/waver) [![Documentation](https://img.shields.io/badge/docs-current-blue.svg?logo=rust)](https://docs.rs/waver)

Waver is a comprehensive no-std library for signal generation, frequency analysis, and waveform synthesis
across the entire electromagnetic spectrum—from sub-Hz geological signals to GHz radio frequencies.

Generate precise sinusoidal and complex waveforms with dynamic modulation, perform spectral analysis with
FFT/STFT, and synthesize signals from recorded data. Whether you're working with audio, RF communications,
sensor signals, or scientific instrumentation, Waver provides the tools for signal processing in embedded
and bare-metal environments like [Arduino] and [Raspberry Pi].

## Installation

To use Waver, add the following to your `Cargo.toml` file.

```toml
[dependencies]
waver = "0.2"
```

## Example

```rust
# extern crate alloc;
use std::{vec::Vec, f32::consts::PI};
use waver::{Waveform, Wave, WaveFunc, Modulation, quantization::quantize_samples};
use alloc::vec;

fn main() {
  // 44.1Khz sampling rate with f32 precision.
  let mut wf = waver::Waveform::<f32>::new(44100.0);

  // Superpose a sine wave, a cosine wave and a triangle function.
  wf.superpose(Wave { frequency: 2600.0, ..Default::default() }).unwrap();
  wf.superpose(Wave { frequency: 2600.0, phase: Modulation::Static(PI / 2.0), ..Default::default() }).unwrap();
  wf.superpose(Wave { frequency: 2600.0, func: WaveFunc::Triangle, ..Default::default() }).unwrap();
  wf.normalize_amplitudes().unwrap();

  // Generate 100 float samples
  let float_output: Vec<f32> = wf.iter().take(100).collect();

  // Quantize to 16-bit integers for output
  let _quantized_output: Vec<i16> = quantize_samples(&float_output);
}
```

## Features

### Signal Generation

- **Arbitrary precision**: Generate signals using `f32`, `f64`, or other float types for your precision requirements
- **Streaming generation**: Infinite iterators with no intermediate buffers
- **Signal superposition**: Combine multiple waveforms with weighted amplitudes
- **Five wave functions**: Sine, Cosine, Square, Sawtooth, Triangle
- **Extreme frequency ranges**: From sub-Hz (geological, tidal) to GHz (RF, microwave)

### Dynamic Modulation

- **LFO (Low-Frequency Oscillator)**: Implement AM/FM modulation, tremolo, vibrato, and other effects
- **Envelope modulation**: Apply time-varying amplitude and phase profiles
- **Static modulation**: Constant-value modulation for simple signals

### Spectral Analysis

- **FFT**: Fast Fourier Transform for frequency-domain analysis of recorded signals
- **STFT**: Short-Time Fourier Transform for time-frequency analysis of non-stationary signals
- **Waveform synthesis**: Reconstruct signals from spectral data with harmonic tracking
- **Universal frequency support**: Analyze signals from sub-Hz oscillations to GHz electromagnetic waves

### Quantization & Data Conversion

- **Flexible bit depths**: Convert between floating-point and any integer type (`i8`, `i16`, `i32`, `u8`, `u16`, etc.)
- **Zero-copy streaming**: Iterator-based quantization for memory efficiency
- **Bidirectional conversion**: Seamless transformation between float and integer representations

### Embedded & Bare-Metal

- **no-std compatible**: Works without the standard library
- **Numerically stable**: Prevents clipping and overflow in signal processing
- **Embedded-friendly**: Tested on Arduino, Raspberry Pi, and other microcontroller platforms
- **Optional alloc**: Dynamic allocations only when needed

## Module Organization

Waver is organized into specialized modules for different signal processing tasks:

- **`wave`**: Core signal generation with `Wave`, `WaveFunc`, and `Modulation` types
- **`waveform`**: Complex signal composition via `Waveform` for both generative and recorded signals
- **`quantization`**: Precision conversion utilities for floating-point and integer representations
- **`analysis`**: Spectral analysis with FFT, STFT, and signal reconstruction capabilities
- **`error`**: Error handling for all operations

## Use Cases

- **Audio Processing**: Sound synthesis, effects, and analysis (20Hz - 20kHz)
- **RF Communications**: Carrier wave generation and modulation (MHz - GHz)
- **Sensor Signal Processing**: Low-frequency sensor data analysis (sub-Hz - kHz)
- **Scientific Instrumentation**: Precision waveform generation for experiments
- **Embedded Systems**: Real-time signal generation on microcontrollers
- **Seismology & Geophysics**: Analysis of ultra-low frequency signals
- **Telecommunications**: Modulated carrier wave synthesis

## Examples

### Generate a High-Frequency RF Carrier (100 MHz)

```rust
use waver::{Wave, Waveform};

// Generate 100 MHz carrier wave (common FM radio frequency)
let rf_carrier = Wave {
    sample_rate: 1_000_000_000.0,  // 1 GHz sample rate (10x Nyquist)
    frequency: 100_000_000.0,       // 100 MHz
    amplitude: 0.8.into(),
    ..Default::default()
};

let waveform = Waveform::<f32>::with_wave(1_000_000_000.0, rf_carrier);
let samples: Vec<f32> = waveform.iter().take(10000).collect();
```

### Analyze Low-Frequency Sensor Data

```rust
use waver::{Waveform, analysis};

// Analyze sub-Hz sensor oscillations (e.g., tidal measurements)
let sensor_data: Vec<f32> = vec![/* your recorded sensor data */];
let waveform = Waveform::<f32>::from_recorded_samples(10.0, &sensor_data);  // 10 Hz sampling

// Extract dominant frequency components
let spectrum = analysis::spectrum(&waveform, 1024);
let (freq, magnitude, phase) = spectrum.data
    .iter()
    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    .unwrap();

println!("Dominant frequency: {:.4} Hz with magnitude {:.2}", freq, magnitude);
```

### Basic Waveform Generation

```rust
use waver::{Waveform, Wave, WaveFunc, Modulation};

// Create a 440Hz sine wave at 44.1kHz sample rate
let wave = Wave {
    sample_rate: 44100.0,
    frequency: 440.0,
    amplitude: Modulation::Static(0.8),
    phase: Modulation::Static(0.0),
    func: WaveFunc::Sine,
};

let waveform = Waveform::<f32>::with_wave(44100.0, wave);
let samples: Vec<f32> = waveform.iter().take(44100).collect(); // 1 second
```

### Wave Superposition

```rust
use waver::{Waveform, Wave};

let mut waveform = Waveform::<f32>::new(44100.0);

// Add fundamental and harmonics
waveform.superpose(Wave { frequency: 220.0, ..Default::default() }).unwrap();
waveform.superpose(Wave { frequency: 440.0, ..Default::default() }).unwrap();
waveform.superpose(Wave { frequency: 660.0, ..Default::default() }).unwrap();

// Normalize amplitudes to prevent clipping
waveform.normalize_amplitudes().unwrap();

let samples: Vec<f32> = waveform.iter().take(44100).collect();
```

### LFO Modulation (Tremolo/Vibrato)

```rust
use waver::{Wave, Waveform, Modulation};

// Create 5Hz LFO for tremolo effect
let lfo = Wave {
    sample_rate: 44100.0,
    frequency: 5.0,
    amplitude: Modulation::Static(0.3),
    ..Default::default()
};

// Apply LFO to carrier wave amplitude
let carrier = Wave {
    sample_rate: 44100.0,
    frequency: 440.0,
    amplitude: Modulation::LFO(Box::new(lfo)),
    ..Default::default()
};

let waveform = Waveform::<f32>::with_wave(44100.0, carrier);
let samples: Vec<f32> = waveform.iter().take(44100).collect();
```

### Signal Analysis and Resynthesis

```rust
use waver::{Waveform, analysis};

// Load or generate recorded samples
let recorded_samples: Vec<f32> = vec![/* your samples */];
let waveform = Waveform::<f32>::from_recorded_samples(44100.0, &recorded_samples);

// Perform FFT analysis
let spectrum = analysis::spectrum(&waveform, 2048);
let (dominant_freq, magnitude, phase) = spectrum.data
    .iter()
    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    .unwrap();

// Perform STFT for time-varying frequency analysis
let spectrogram = analysis::time_spectrum(&waveform, 2048, 512).unwrap();

// Examine how frequency content evolves over time
for (frame_idx, spectrum) in spectrogram.iter().enumerate() {
    let time = frame_idx as f32 * 512.0 / 44100.0;
    let (peak_freq, magnitude, _) = spectrum.data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();

    println!("t={:.3}s: peak at {:.1}Hz (mag: {:.2})", time, peak_freq, magnitude);
}

// Resynthesize a generative waveform from recorded samples
let synthesized = analysis::synthesize(&waveform, 2048, 512, 10).unwrap();
let new_samples: Vec<f32> = synthesized.iter().take(44100).collect();
```

### Quantization Examples

```rust
use waver::{Wave, Waveform, quantization::{quantize_samples, dequantize_samples, QuantizeIterator}};

// Generate float samples
let wave = Wave { frequency: 440.0, ..Default::default() };
let waveform = Waveform::<f32>::with_wave(44100.0, wave);

// Method 1: Direct quantization
let float_samples: Vec<f32> = waveform.iter().take(100).collect();
let int_samples: Vec<i16> = quantize_samples(&float_samples);

// Method 2: Iterator-based quantization (zero-copy)
let int_samples: Vec<i16> = waveform.iter().take(100).quantize().collect();

// Dequantize back to floats
let recovered_floats: Vec<f32> = dequantize_samples(&int_samples);
```

## TODO

- [ ] Implement checks to protect against aliasing (e.g., disallow frequencies above the Nyquist frequency).
- [ ] Use fixed-point arithmetic for platforms that doesn't have an FPU.
- [ ] Replace use of libm crate [when math support moves to libcore].
- [ ] Add support for more advanced windowing functions for STFT.
- [ ] Expand inverse FFT capabilities for complete signal reconstruction.

## Contributing

Thought of something you'd like to see in Waver? You can visit the issue tracker
to check if it was reported or proposed before, and if not please feel free to
create an issue or feature request. Ready to start contributing?
The [contributing guide][contributing] is a good place to start. If you have
questions please feel free to ask.

[Arduino]: https://www.arduino.cc/
[Raspberry Pi]: https://www.raspberrypi.org/
[contributing]: https://github.com/amrali/waver/blob/master/CONTRIBUTING.md
[when math support moves to libcore]: https://github.com/rust-lang/rust/issues/137578
