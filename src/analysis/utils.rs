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

//! Utility functions for signal analysis.

use super::spectrum::Spectrum;
use crate::{Modulation, Wave};
use alloc::vec::Vec;
use core::f32::consts::PI;

/// Advanced spectral peak with sub-bin frequency accuracy
#[derive(Debug, Clone)]
pub struct SpectralPeak {
    pub frequency: f32,
    pub magnitude: f32,
    pub phase: f32,
}

/// A frequency track that follows a spectral component across time
#[derive(Debug, Clone)]
pub struct FrequencyTrack {
    frames: Vec<Option<FrameData>>,
    confidence: f32,
    last_frequency: f32,
}

#[derive(Debug, Clone)]
struct FrameData {
    frequency: f32,
    magnitude: f32,
    phase: f32,
}

/// Advanced peak detection with parabolic interpolation for sub-bin accuracy
pub fn find_spectral_peaks_advanced(
    spectrum: &Spectrum,
    max_peaks: usize,
    frequency_resolution: f32,
) -> Vec<SpectralPeak> {
    let spectrum_data = &spectrum.data;

    if spectrum_data.len() < 3 {
        return Vec::new();
    }

    // Calculate dynamic magnitude threshold based on spectrum statistics
    let magnitudes: Vec<f32> = spectrum_data
        .iter()
        .map(|(_, magnitude, _)| *magnitude)
        .collect();
    let max_magnitude = magnitudes.iter().fold(0.0f32, |acc, &val| acc.max(val));
    let mean_magnitude = magnitudes.iter().sum::<f32>() / magnitudes.len() as f32;

    // Adaptive threshold: use a fraction of max or above mean, whichever is lower
    let magnitude_threshold = (max_magnitude * 0.01).min(mean_magnitude * 2.0).max(0.0001);

    // Find local maxima with magnitude threshold
    let mut peaks: Vec<SpectralPeak> = spectrum_data
        .windows(3)
        .filter_map(|window| {
            let (frequency, magnitude, phase) = window[1]; // Center element

            // Check if it's a local maximum above threshold
            if magnitude > magnitude_threshold && magnitude > window[0].1 && magnitude > window[2].1
            {
                // Parabolic interpolation for sub-bin frequency accuracy
                let (y_prev, y_curr, y_next) = (window[0].1, magnitude, window[2].1);

                // Avoid division by zero
                let parabola_a = (y_prev - 2.0 * y_curr + y_next) / 2.0;
                let parabola_b = (y_next - y_prev) / 2.0;

                let bin_offset = if parabola_a.abs() > 1e-10 {
                    -parabola_b / (2.0 * parabola_a)
                } else {
                    0.0
                };
                let bin_offset = bin_offset.clamp(-0.5, 0.5); // Limit offset to reasonable range

                let interpolated_frequency = frequency + bin_offset * frequency_resolution;
                let interpolated_magnitude = y_curr - (parabola_b * bin_offset / 2.0);

                Some(SpectralPeak {
                    frequency: interpolated_frequency.max(0.0),
                    magnitude: interpolated_magnitude.max(0.0),
                    phase,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by magnitude (strongest first) and take only the requested number
    peaks.sort_by(|a, b| {
        b.magnitude
            .partial_cmp(&a.magnitude)
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    peaks.truncate(max_peaks);
    peaks
}

/// Check if a peak can be matched to an existing track
pub fn can_match_to_track(
    track: &FrequencyTrack,
    peak: &SpectralPeak,
    _frame_index: usize,
    frequency_resolution: f32,
) -> bool {
    let expected_frequency = track.last_frequency;
    let base_tolerance = calculate_frequency_tolerance(expected_frequency, frequency_resolution);

    // Allow larger tolerance for very low frequencies and high frequencies
    let adaptive_tolerance = match expected_frequency {
        freq if freq < 1.0 => base_tolerance * 10.0, // Very lenient for sub-Hz
        freq if freq > 10000.0 => base_tolerance * 2.0, // More lenient for high frequencies
        _ => base_tolerance,
    };

    (peak.frequency - expected_frequency).abs() < adaptive_tolerance
}

/// Calculate frequency tolerance for track matching
pub fn calculate_frequency_tolerance(frequency: f32, freq_resolution: f32) -> f32 {
    if frequency < 1.0 {
        // For very low frequencies, use absolute tolerance
        0.5f32.max(freq_resolution)
    } else if frequency < 100.0 {
        // For low frequencies, use 5% tolerance
        frequency * 0.05
    } else {
        // For higher frequencies, use smaller percentage but not less than freq resolution
        (frequency * 0.02).max(freq_resolution * 2.0)
    }
}

/// Universal amplitude scaling that works for all frequency ranges
pub fn calculate_universal_amplitude_scaling(
    amplitudes: &[f32],
    window_size: usize,
    _sample_rate: f32,
    frequency: f32,
) -> f32 {
    if amplitudes.is_empty() {
        return 1.0;
    }

    let max_amplitude = amplitudes.iter().fold(0.0f32, |acc, &val| acc.max(val));
    if max_amplitude <= 0.0 {
        return 1.0;
    }

    // Base scaling factor
    let base_scale = 2.0 / window_size as f32;

    // Frequency-dependent scaling using pattern matching for clarity
    let frequency_scale = match frequency {
        freq if freq < 1.0 => 100.0, // Very low frequencies need more amplification
        freq if freq < 10.0 => 50.0, // Low frequencies
        freq if freq < 1000.0 => 20.0, // Audio frequencies
        freq if freq < 10000.0 => 10.0, // High audio frequencies
        _ => 5.0,                    // Very high frequencies
    };

    // Magnitude-dependent scaling using pattern matching
    let magnitude_scale = match max_amplitude {
        mag if mag > 100000.0 => 0.0001,
        mag if mag > 10000.0 => 0.001,
        mag if mag > 1000.0 => 0.01,
        _ => 0.1,
    };

    base_scale * frequency_scale * magnitude_scale
}

/// Unwrap phase to ensure continuity
pub fn unwrap_phase(phases: &[f32]) -> Vec<f32> {
    if phases.is_empty() {
        return Vec::new();
    }

    let mut unwrapped = Vec::with_capacity(phases.len());
    unwrapped.push(phases[0]);

    let mut cumulative_offset = 0.0f32;

    for i in 1..phases.len() {
        let mut phase = phases[i] + cumulative_offset;
        let diff = phase - unwrapped[i - 1];

        // Unwrap phase jumps greater than π
        if diff > PI {
            cumulative_offset -= 2.0 * PI;
            phase -= 2.0 * PI;
        } else if diff < -PI {
            cumulative_offset += 2.0 * PI;
            phase += 2.0 * PI;
        }

        unwrapped.push(phase);
    }

    unwrapped
}

impl FrequencyTrack {
    pub const fn new(initial_frequency: f32) -> Self {
        Self {
            frames: Vec::new(),
            confidence: 1.0,
            last_frequency: initial_frequency,
        }
    }

    pub fn add_frame(&mut self, frame_index: usize, frequency: f32, magnitude: f32, phase: f32) {
        // Extend frames vector if necessary
        self.frames.resize(frame_index + 1, None);

        // Update confidence based on frequency stability
        if !self.frames.is_empty() {
            let frequency_deviation = (frequency - self.last_frequency).abs();
            let relative_deviation = frequency_deviation / self.last_frequency.max(1.0);
            self.confidence *= (1.0 - relative_deviation * 0.1).max(0.1);
        }

        self.frames[frame_index] = Some(FrameData {
            frequency,
            magnitude,
            phase,
        });

        self.last_frequency = frequency;
    }

    pub fn fill_gaps_to_frame(&mut self, frame_index: usize) {
        self.frames.resize(frame_index + 1, None);
    }

    pub fn is_valid(&self, min_length: usize) -> bool {
        let active_frame_count = self.frames.iter().filter(|frame| frame.is_some()).count();
        active_frame_count >= min_length && self.confidence > 0.1
    }

    pub fn to_wave(&self, window_size: usize, sample_rate: f32) -> Option<Wave> {
        let frame_data: Vec<&FrameData> = self.frames.iter().filter_map(Option::as_ref).collect();

        if frame_data.is_empty() {
            return None;
        }

        // Calculate representative frequency using weighted average
        let total_weight: f32 = frame_data.iter().map(|frame| frame.magnitude).sum();
        if total_weight <= 0.0 {
            return None;
        }

        let weighted_frequency: f32 = frame_data
            .iter()
            .map(|frame| frame.frequency * frame.magnitude)
            .sum::<f32>()
            / total_weight;

        // Create amplitude and phase envelopes
        let (amplitude_envelope, phase_envelope): (Vec<f32>, Vec<f32>) = self
            .frames
            .iter()
            .scan(0.0f32, |last_phase, frame_option| match frame_option {
                Some(data) => {
                    *last_phase = data.phase;
                    Some((data.magnitude, data.phase))
                }
                None => Some((0.0, *last_phase)), // Interpolate missing frames
            })
            .unzip();

        // Apply amplitude scaling that works for all frequency ranges
        let amplitude_scale = calculate_universal_amplitude_scaling(
            &amplitude_envelope,
            window_size,
            sample_rate,
            weighted_frequency,
        );

        let normalized_amplitude: Vec<f32> = amplitude_envelope
            .iter()
            .map(|&amplitude| amplitude * amplitude_scale * self.confidence)
            .collect();

        // Unwrap phase for continuity
        let unwrapped_phase = unwrap_phase(&phase_envelope);

        Some(Wave {
            frequency: weighted_frequency,
            amplitude: Modulation::Envelope(normalized_amplitude),
            phase: Modulation::Envelope(unwrapped_phase),
            ..Default::default()
        })
    }
}
