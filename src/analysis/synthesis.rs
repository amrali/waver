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
use crate::{error::Error, waveform::Waveform, Wave};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Synthesizes a generative `Waveform` from a recorded `Waveform`.
///
/// Uses advanced spectral analysis to extract frequency components and reconstruct
/// them as a generative waveform. This algorithm supports all frequency ranges
/// from sub-Hz to MHz/GHz frequencies with appropriate sample rates.
///
/// # Algorithm Features:
/// - Dynamic frequency resolution based on sample rate and frequency content
/// - Logarithmic frequency binning for better low-frequency resolution
/// - Peak detection with sub-bin accuracy using parabolic interpolation
/// - Adaptive amplitude scaling across different magnitude ranges
/// - Support for extreme frequency ranges with proper configuration
///
/// # Arguments
///
/// * `waveform` - The recorded waveform to synthesize.
/// * `window_size` - The number of samples in each analysis window.
/// * `hop_size` - The number of samples to hop between windows.
/// * `max_harmonics` - The maximum number of harmonics to track per frame.
///
/// # Returns
///
/// A `Result` containing a new generative `Waveform`.
pub fn synthesize<BitDepth: Clone>(
    waveform: &Waveform<BitDepth>,
    window_size: usize,
    hop_size: usize,
    max_harmonics: usize,
) -> Result<Waveform<BitDepth>, Error> {
    let spectrogram = time_spectrum(waveform, window_size, hop_size)?;

    if spectrogram.is_empty() {
        return Ok(Waveform::new(waveform.sample_rate));
    }

    let frequency_resolution = waveform.sample_rate / window_size as f32;

    // Use direct frequency tracking instead of binning for extreme accuracy
    let mut frequency_tracks: BTreeMap<u64, utils::FrequencyTrack> = BTreeMap::new();
    let mut next_track_id = 0u64;

    // Process each frame of the spectrogram
    for (frame_index, spectrum) in spectrogram.iter().enumerate() {
        // Find spectral peaks with sub-bin accuracy
        let peaks =
            utils::find_spectral_peaks_advanced(spectrum, max_harmonics, frequency_resolution);

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

    // Convert frequency tracks to waves with improved filtering
    let min_track_length = (spectrogram.len() / 16).max(2); // More lenient for extreme frequencies
    let waves: Vec<Wave> = frequency_tracks
        .into_values()
        .filter(|track| track.is_valid(min_track_length))
        .filter_map(|track| track.to_wave(window_size, waveform.sample_rate))
        .collect();

    // Create final waveform
    let mut synthesized_waveform = Waveform::new(waveform.sample_rate);
    for wave in waves {
        synthesized_waveform.superpose(wave)?;
    }

    Ok(synthesized_waveform)
}
