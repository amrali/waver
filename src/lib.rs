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
//! use std::{vec::Vec, f32::consts::PI};
//! use waver::{Waveform, Wave};
//!
//! fn main() {
//!   // 44.1Khz sampling rate and 16-bit depth.
//!   let mut wf = waver::Waveform::<i16>::new(44100.0);
//!
//!   // Superpose a sine wave and a cosine wave.
//!   wf.superpose(Wave { frequency: 2600.0, ..Default::default() })
//!     .superpose(Wave { frequency: 2600.0, phase: PI / 2.0, ..Default::default() })
//!     .normalize_amplitudes();
//!
//!   // Quantization of 100 samples
//!   let _output: Vec<i16> = wf.iter().take(100).collect();
//! }
//! ```

#![no_std]

extern crate alloc;

// Use wee_alloc as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

mod wave;
mod waveform;

pub use wave::{Wave, WaveFunc, WaveIterator};
pub use waveform::{Waveform, WaveformIterator};

// Test README.md
use doc_comment::doctest;
doctest!("../README.md");
