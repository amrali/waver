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

//! # Waver: waveform generation library
//!
//! A waveform can be the fundamental sinusoidal wave or a complex waveform of varying
//! frequency, phase shift or amplitude. Waver is useful where there's a need to generate
//! a simple sinusoidal sound wave or for constructing a frequency or amplitude
//! modulated carrier wave in bare-metal Arduino or a Raspberry Pi.
//!
//! ## Features
//!
//! * Arbitrary quantization levels. Specify the bit depth when constructing `Waveform`.
//! * Online wave generation. No buffers, infinite iterators.
//! * Wave superposition with weighted amplitudes.
//! * Modulate signal's frequency, amplitude or phase.
//! * Numerically stable, prevents clipping.
//!
//! ## Example
//!
//! ```rust
//! # extern crate alloc;
//! use std::{vec::Vec, f32::consts::PI};
//! use waver::{Waveform, Wave, WaveFunc, quantization::quantize_samples};
//! use alloc::vec;
//!
//! // 44.1Khz sampling rate with f32 precision.
//! let mut wf = waver::Waveform::<f32>::new(44100.0);
//!
//! // Superpose a sine wave, a cosine wave and a triangle function.
//! wf.superpose(Wave { frequency: 2600.0, ..Default::default() }).unwrap();
//! wf.superpose(Wave { frequency: 2600.0, phase: waver::Modulation::Static(PI / 2.0), ..Default::default() }).unwrap();
//! wf.superpose(Wave { frequency: 2600.0, func: WaveFunc::Triangle, ..Default::default() }).unwrap();
//! wf.normalize_amplitudes().unwrap();
//!
//! // Generate 100 float samples
//! let float_output: Vec<f32> = wf.iter().take(100).collect();
//!
//! // Quantize to 16-bit integers
//! let quantized_output: Vec<i16> = quantize_samples(&float_output);
//! ```

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod analysis;
pub mod error;
pub mod quantization;
mod wave;
mod waveform;

pub use self::error::Error;
pub use quantization::{
    dequantize_samples, quantize_samples, DequantizationIterator, DequantizeIterator,
    QuantizationIterator, QuantizeIterator,
};
pub use wave::{Modulation, Wave, WaveFunc, WaveIterator};
pub use waveform::{Waveform, WaveformIterator, WaveformSource};

// Test README.md
#[cfg(doctest)]
mod readme {
    use doc_comment::doctest;
    doctest!("../README.md");
}
