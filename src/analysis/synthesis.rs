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

//! Waveform synthesis from spectral analysis.

use super::{stft::time_spectrum, utils};
use crate::{error::Error, waveform::Waveform};
use alloc::collections::BTreeMap;

/// Synthesizes a generative `Waveform` from a recorded `Waveform`.
///
/// Uses advanced spectral analysis to extract frequency components and reconstruct
/// them as a generative waveform. This algorithm supports all frequency ranges
/// from sub-Hz to MHz/GHz frequencies with appropriate sample rates.
///
/// # Algorithm Features:
/// - Dynamic frequency resolution based on sample rate and frequency content
/// - Peak detection with sub-bin accuracy using parabolic interpolation
/// - Adaptive amplitude scaling across different magnitude ranges
/// - Support for extreme frequency ranges with proper configuration
/// - Fully generic over float types with maintained precision
///
/// # Arguments
///
/// * `waveform` - The recorded waveform to synthesize.
/// * `window_size` - The number of samples in each analysis window (must be ≥ 4).
/// * `hop_size` - The number of samples to hop between windows (must be > 0).
/// * `max_harmonics` - The maximum number of harmonics to track per frame.
///
/// # Errors
///
/// Returns `Error::InvalidParameters` if:
/// - `window_size` is too small (< 4 samples) for meaningful spectral analysis
/// - `hop_size` is zero (would cause infinite loop)
/// - `sample_rate` is zero or negative
/// - `window_size` is excessively large (> 1,000,000 samples)
///
/// Returns `Error::UnsupportedSource` if the waveform source type is not supported.
///
/// # Returns
///
/// A `Result` containing a new generative `Waveform`.
pub fn synthesize<BitDepth>(
    waveform: &Waveform<BitDepth>,
    window_size: usize,
    hop_size: usize,
    max_harmonics: usize,
) -> Result<Waveform<BitDepth>, Error>
where
    BitDepth: num_traits::Float
        + Copy
        + core::iter::Sum
        + Clone
        + rustfft::FftNum
        + core::fmt::Debug
        + Default
        + num_traits::FromPrimitive,
{
    // Parameter validation
    if window_size < 4 {
        return Err(Error::InvalidParameters {
            message: "window_size must be at least 4 samples for FFT analysis",
        });
    }

    if hop_size == 0 {
        return Err(Error::InvalidParameters {
            message: "hop_size must be greater than 0 to avoid infinite loops",
        });
    }

    // Check for reasonable sample rate
    if waveform.sample_rate <= BitDepth::zero() {
        return Err(Error::InvalidParameters {
            message: "sample_rate must be positive",
        });
    }

    // For very large windows, ensure we can still do meaningful analysis
    // Allow large windows for sub-Hz analysis, but check for extreme cases
    if window_size > 1_000_000 {
        return Err(Error::InvalidParameters {
            message: "window_size exceeds reasonable limits (> 1,000,000 samples)",
        });
    }

    let spectrogram = time_spectrum(waveform, window_size, hop_size)?;

    if spectrogram.is_empty() {
        return Ok(Waveform::new(waveform.sample_rate));
    }

    let frequency_resolution: BitDepth =
        waveform.sample_rate / BitDepth::from(window_size).unwrap_or_else(|| BitDepth::one());

    // Use direct frequency tracking instead of binning for extreme accuracy
    let mut frequency_tracks: BTreeMap<u64, utils::FrequencyTrack<BitDepth>> = BTreeMap::new();
    let mut next_track_id = 0u64;

    // Process each frame of the spectrogram
    for (frame_index, spectrum) in spectrogram.iter().enumerate() {
        // Find spectral peaks with sub-bin accuracy
        let peaks = utils::find_spectral_peaks(spectrum, max_harmonics, frequency_resolution);

        // Track peaks across time frames
        for peak in peaks {
            let mut peak_matched = false;

            // Try to match with existing tracks
            for track in frequency_tracks.values_mut() {
                if utils::can_match_to_track(track, &peak, frame_index, frequency_resolution) {
                    track.add_frame(frame_index, peak.frequency, peak.magnitude, peak.phase);
                    peak_matched = true;
                    break;
                }
            }

            // Create new track if no match found
            if !peak_matched {
                let mut new_track = utils::FrequencyTrack::new(peak.frequency);
                new_track.add_frame(frame_index, peak.frequency, peak.magnitude, peak.phase);
                frequency_tracks.insert(next_track_id, new_track);
                next_track_id += 1;
            }
        }

        // Fill gaps in existing tracks
        for track in frequency_tracks.values_mut() {
            track.fill_gaps_to_frame(frame_index);
        }
    }

    // Convert frequency tracks to waves
    let min_track_length = (spectrogram.len() / 16).max(2);
    let mut synthesized_waveform = Waveform::new(waveform.sample_rate);

    for track in frequency_tracks.into_values() {
        if track.is_valid(min_track_length) {
            // Use the generic to_wave method that returns Wave<BitDepth>
            if let Some(wave) = track.to_wave(window_size, waveform.sample_rate) {
                if synthesized_waveform.superpose(wave).is_err() {
                    // Continue with other waves if one fails
                    continue;
                }
            }
        }
    }

    Ok(synthesized_waveform)
}
