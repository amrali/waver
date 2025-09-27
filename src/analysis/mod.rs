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

//! Signal analysis module for `no-std` environments.
//!
//! Provides FFT spectrum analysis, STFT spectrograms, and waveform synthesis.

mod spectrum;
mod stft;
mod synthesis;
mod utils;

// Re-export public API
pub use spectrum::{spectrum, Spectrum};
pub use stft::time_spectrum;
pub use synthesis::synthesize;

#[cfg(test)]
mod tests;
