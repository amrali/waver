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

//! A module for the library's error types.

//! This module contains the unified error type for the `waver` library,
//! allowing for consistent and predictable error handling across all modules.

/// The error type for the waver library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// An operation was attempted on a waveform with an unsupported source.
    /// For example, trying to superpose a wave on a recorded waveform.
    UnsupportedSource,

    /// Invalid analysis parameters were provided.
    InvalidParameters {
        /// Description of the parameter error.
        message: &'static str,
    },
}
