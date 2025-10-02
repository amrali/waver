# Waver [![CI](https://github.com/amrali/waver/actions/workflows/main.yml/badge.svg)](https://github.com/amrali/waver/actions/workflows/main.yml) [![codecov](https://codecov.io/gh/amrali/waver/branch/master/graph/badge.svg?token=fN3pEuLaAB)](https://codecov.io/gh/amrali/waver) [![Crates.io](https://img.shields.io/crates/v/waver.svg?logo=rust)](https://crates.io/crates/waver) [![Documentation](https://img.shields.io/badge/docs-current-blue.svg?logo=rust)](https://docs.rs/waver)

Waver is a comprehensive no-std library for waveform generation, signal analysis, and audio synthesis.
Generate waveforms with dynamic modulation, analyze recorded signals with FFT/STFT, and resynthesize
audio from frequency analysis.

A waveform can be a simple sinusoidal wave or a complex waveform of varying
frequency and amplitude. Waver is useful where there's a need to generate
a simple sinusoidal sound wave or for constructing a frequency or amplitude
modulated carrier wave in bare-metal [Arduino] or a [Raspberry Pi].

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

### Waveform Generation

- **Arbitrary precision**: Generate waveforms using `f32`, `f64`, or other float types
- **Online generation**: No buffers, infinite iterators
- **Wave superposition**: Combine multiple waves with weighted amplitudes
- **Five wave functions**: Sine, Cosine, Square, Sawtooth, Triangle

### Dynamic Modulation

- **LFO (Low-Frequency Oscillator)**: Create tremolo, vibrato, and other modulation effects
- **Envelope modulation**: Apply custom amplitude and phase envelopes over time
- **Static values**: Use constant amplitude and phase for simple waveforms

### Signal Analysis

- **FFT**: Perform frequency analysis on recorded signals to extract frequency spectrum
- **STFT**: Short-Time Fourier Transform for time-varying frequency analysis
- **Dynamic synthesis**: Resynthesize waveforms from recorded samples by analyzing their harmonic structure
- **Sub-Hz to GHz**: Support for extreme frequency ranges with appropriate sample rates

### Quantization

- **Flexible bit depths**: Convert float samples to any integer type (`i8`, `i16`, `i32`, `u8`, `u16`, etc.)
- **Iterator-based**: Stream quantization with zero-copy performance
- **Bidirectional**: Convert between floats and integers seamlessly

### No-std Compatible

- Numerically stable clipping prevention
- Works on embedded systems, Arduino, Raspberry Pi, and other bare-metal environments
- Optional `alloc` support for dynamic allocations

## Module Organization

Waver is organized into several public modules for different functionality:

- **`wave`**: Core wave generation with `Wave`, `WaveFunc`, and `Modulation` types
- **`waveform`**: Complex waveform composition via `Waveform` for generative and recorded signals
- **`quantization`**: Sample quantization utilities for converting between float and integer representations
- **`analysis`**: Signal analysis with FFT, STFT, and waveform synthesis capabilities
- **`error`**: Error types for operation failures

## Examples

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

// Perform STFT for time-varying analysis
let spectrogram = analysis::time_spectrum(&waveform, 2048, 512).unwrap();

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
