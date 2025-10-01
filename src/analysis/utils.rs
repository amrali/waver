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

//! Signal analysis utilities for spectral processing and frequency tracking.

use super::spectrum::Spectrum;
use crate::Wave;
use alloc::vec::Vec;

/// A spectral peak with sub-bin frequency accuracy using parabolic interpolation.
#[derive(Debug, Clone)]
pub struct SpectralPeak<F> {
    pub frequency: F,
    pub magnitude: F,
    pub phase: F,
}

/// Tracks a frequency component across multiple time frames for synthesis.
#[derive(Debug, Clone)]
pub struct FrequencyTrack<F> {
    frames: Vec<Option<FrameData<F>>>,
    confidence: F,
    last_frequency: F,
}

#[derive(Debug, Clone)]
struct FrameData<F> {
    frequency: F,
    magnitude: F,
    phase: F,
}

/// Finds spectral peaks using adaptive thresholding and parabolic interpolation.
///
/// Uses a two-tier threshold system:
/// - Base threshold: 1% of maximum magnitude
/// - Noise floor: 3x mean magnitude
///
/// Applies parabolic interpolation for sub-bin frequency accuracy and removes
/// peaks closer than 1.5 bins to avoid spectral leakage artifacts.
pub fn find_spectral_peaks<F>(
    spectrum: &Spectrum<F>,
    max_peaks: usize,
    freq_resolution: F,
) -> Vec<SpectralPeak<F>>
where
    F: num_traits::Float + Copy + PartialOrd + core::fmt::Debug,
{
    if spectrum.data.len() < 3 {
        return Vec::new();
    }

    // Calculate adaptive threshold based on spectrum statistics
    let magnitudes: Vec<F> = spectrum.data.iter().map(|(_, mag, _)| *mag).collect();
    let max_magnitude = magnitudes.iter().fold(F::zero(), |acc, &val| acc.max(val));
    let mean_magnitude = magnitudes.iter().fold(F::zero(), |acc, &val| acc + val)
        / F::from(magnitudes.len()).unwrap();

    let threshold =
        (max_magnitude * F::from(0.01).unwrap()).max(mean_magnitude * F::from(3.0).unwrap());

    // Find local maxima using 3-point windows
    let mut peaks: Vec<SpectralPeak<F>> = spectrum
        .data
        .windows(3)
        .filter_map(|window| {
            let (_, magnitude, phase) = window[1];

            // Must exceed threshold
            if magnitude <= threshold {
                return None;
            }

            // Must be local maximum
            if magnitude <= window[0].1 || magnitude <= window[2].1 {
                return None;
            }

            // For weaker peaks, require clearer dominance to reduce false positives
            let is_strong_peak = magnitude > mean_magnitude * F::from(20.0).unwrap();
            if !is_strong_peak {
                let margin = F::from(1.1).unwrap(); // 10% margin
                if magnitude < window[0].1 * margin || magnitude < window[2].1 * margin {
                    return None;
                }
            }

            let interpolated_freq = interpolate_peak_frequency(window, freq_resolution);

            // Reject DC and very low frequencies that are likely DC offset
            if interpolated_freq < F::from(0.1).unwrap() {
                return None;
            }

            Some(SpectralPeak {
                frequency: interpolated_freq.max(F::zero()),
                magnitude,
                phase,
            })
        })
        .collect();

    // Sort by magnitude (strongest first)
    peaks.sort_by(|a, b| {
        b.magnitude
            .partial_cmp(&a.magnitude)
            .unwrap_or(core::cmp::Ordering::Equal)
    });

    remove_close_peaks(peaks, freq_resolution, max_peaks)
}

/// Applies parabolic interpolation to estimate peak frequency with sub-bin accuracy.
///
/// Uses a 3-point parabolic fit around the peak to estimate the true frequency
/// between FFT bins, improving frequency resolution beyond the bin spacing.
fn interpolate_peak_frequency<F>(window: &[(F, F, F)], freq_resolution: F) -> F
where
    F: num_traits::Float + Copy,
{
    let (frequency, magnitude, _) = window[1];
    let (y_prev, y_curr, y_next) = (window[0].1, magnitude, window[2].1);
    let two = F::from(2.0).unwrap();

    // Parabolic interpolation coefficients
    let parabola_a = (y_prev - two * y_curr + y_next) / two;
    let parabola_b = (y_next - y_prev) / two;

    // Calculate bin offset (-0.5 to +0.5 bins)
    let bin_offset = if parabola_a.abs() > F::from(1e-10).unwrap() && parabola_a < F::zero() {
        (-parabola_b / (two * parabola_a)).clamp(F::from(-0.5).unwrap(), F::from(0.5).unwrap())
    } else {
        F::zero()
    };

    frequency + bin_offset * freq_resolution
}

/// Removes peaks that are too close together to avoid spectral leakage artifacts.
///
/// Maintains minimum separation of 1.5 bins between peaks, keeping the strongest
/// peak when multiple peaks are within the separation threshold.
fn remove_close_peaks<F>(
    peaks: Vec<SpectralPeak<F>>,
    freq_resolution: F,
    max_peaks: usize,
) -> Vec<SpectralPeak<F>>
where
    F: num_traits::Float + Copy,
{
    let mut filtered = Vec::new();
    let min_separation = freq_resolution * F::from(1.5).unwrap(); // 1.5 bins minimum

    for peak in peaks {
        let too_close = filtered.iter().any(|existing: &SpectralPeak<F>| {
            (peak.frequency - existing.frequency).abs() < min_separation
        });

        if !too_close {
            filtered.push(peak);
        }

        if filtered.len() >= max_peaks {
            break;
        }
    }

    filtered
}

/// Determines if a spectral peak can be matched to an existing frequency track.
///
/// Uses adaptive tolerance based on frequency range:
/// - Sub-Hz frequencies: 10x base tolerance (very lenient)
/// - High frequencies (>10kHz): 2x base tolerance
/// - Mid frequencies: base tolerance
pub fn can_match_to_track<F>(
    track: &FrequencyTrack<F>,
    peak: &SpectralPeak<F>,
    _frame_index: usize,
    freq_resolution: F,
) -> bool
where
    F: num_traits::Float + Copy + PartialOrd + num_traits::FromPrimitive,
{
    let tolerance = calculate_frequency_tolerance(track.last_frequency, freq_resolution);
    let adaptive_tolerance = if track.last_frequency < F::one() {
        tolerance * F::from(10.0).unwrap() // Very lenient for sub-Hz
    } else if track.last_frequency > F::from(10000.0).unwrap() {
        tolerance * F::from(2.0).unwrap() // More lenient for high frequencies
    } else {
        tolerance
    };

    (peak.frequency - track.last_frequency).abs() < adaptive_tolerance
}

/// Calculates frequency tolerance for track matching based on frequency range.
///
/// Uses different strategies for different frequency ranges:
/// - Sub-Hz: Absolute tolerance (0.5Hz minimum)
/// - Low frequencies (<100Hz): 5% relative tolerance
/// - Higher frequencies: 2% relative tolerance (minimum 2x freq resolution)
pub fn calculate_frequency_tolerance<F>(frequency: F, freq_resolution: F) -> F
where
    F: num_traits::Float + Copy + PartialOrd,
{
    if frequency < F::one() {
        // For very low frequencies, use absolute tolerance
        F::from(0.5).unwrap().max(freq_resolution)
    } else if frequency < F::from(100.0).unwrap() {
        // For low frequencies, use 5% tolerance
        frequency * F::from(0.05).unwrap()
    } else {
        // For higher frequencies, use smaller percentage but not less than freq resolution
        (frequency * F::from(0.02).unwrap()).max(freq_resolution * F::from(2.0).unwrap())
    }
}

/// Calculates amplitude scaling for synthesis that works across all frequency ranges.
///
/// Applies frequency-dependent scaling to compensate for:
/// - Window function energy loss (base scaling)
/// - Frequency-dependent detection sensitivity
/// - Magnitude-dependent normalization
/// - Anti-aliasing near Nyquist frequency
///
/// The scaling is conservative to minimize spurious harmonics while preserving
/// energy in the fundamental frequency.
pub fn calculate_amplitude_scaling<F>(
    amplitudes: &[F],
    window_size: usize,
    sample_rate: F,
    frequency: F,
) -> F
where
    F: num_traits::Float + Copy + PartialOrd + num_traits::FromPrimitive,
{
    if amplitudes.is_empty() {
        return F::one();
    }

    let max_amplitude = amplitudes.iter().fold(F::zero(), |acc, &val| acc.max(val));
    if max_amplitude <= F::zero() {
        return F::one();
    }

    // Base scaling compensates for window function energy loss
    let base_scale = F::from(2.0).unwrap() / F::from(window_size).unwrap();

    // Frequency-dependent scaling - higher for low frequencies that are harder to detect
    let frequency_scale = if frequency < F::from(0.1).unwrap() {
        F::from(20.0).unwrap() // Very low frequencies need significant boost
    } else if frequency < F::from(10.0).unwrap() {
        F::from(12.0).unwrap() // Low frequencies need moderate boost
    } else if frequency < F::from(1000.0).unwrap() {
        F::from(5.0).unwrap() // Audio frequencies
    } else if frequency < F::from(10000.0).unwrap() {
        F::from(3.0).unwrap() // High audio frequencies
    } else {
        F::from(2.0).unwrap() // Very high frequencies
    };

    // Magnitude-dependent scaling to normalize different signal strengths
    let magnitude_scale = if max_amplitude > F::from(100000.0).unwrap() {
        F::from(0.001).unwrap() // Very strong signals need reduction
    } else if max_amplitude > F::from(10000.0).unwrap() {
        F::from(0.01).unwrap()
    } else if max_amplitude > F::from(1000.0).unwrap() {
        F::from(0.1).unwrap()
    } else {
        F::from(0.5).unwrap() // Conservative scaling for normal signals
    };

    // Anti-aliasing factor reduces amplitude near Nyquist frequency
    let nyquist_freq = sample_rate / F::from(2.0).unwrap();
    let anti_alias_factor = if frequency > nyquist_freq * F::from(0.8).unwrap() {
        F::from(0.1).unwrap() // Strongly attenuate near Nyquist
    } else {
        F::one()
    };

    base_scale * frequency_scale * magnitude_scale * anti_alias_factor
}

/// Unwraps phase values to ensure continuity across time frames.
///
/// Detects phase jumps greater than 1.5π and adds/subtracts 2π to maintain
/// continuity. This is essential for proper phase tracking in synthesis.
/// After unwrapping, applies light smoothing to reduce phase noise.
pub fn unwrap_phase<F>(phases: &[F]) -> Vec<F>
where
    F: num_traits::Float + Copy + PartialOrd + num_traits::FromPrimitive,
{
    if phases.is_empty() {
        return Vec::new();
    }

    if phases.len() == 1 {
        return Vec::from([phases[0]]);
    }

    let mut unwrapped = Vec::with_capacity(phases.len());
    unwrapped.push(phases[0]);

    let mut cumulative_offset = F::zero();
    let pi = F::from(core::f64::consts::PI).unwrap();
    let two_pi = F::from(2.0).unwrap() * pi;
    let threshold = pi + pi / F::from(2.0).unwrap(); // 1.5π threshold

    // Unwrap phase jumps
    for i in 1..phases.len() {
        let mut phase = phases[i] + cumulative_offset;
        let diff = phase - unwrapped[i - 1];

        if diff > threshold {
            cumulative_offset = cumulative_offset - two_pi;
            phase = phase - two_pi;
        } else if diff < -threshold {
            cumulative_offset = cumulative_offset + two_pi;
            phase = phase + two_pi;
        }

        unwrapped.push(phase);
    }

    smooth_phase(&unwrapped)
}

/// Applies light smoothing to phase values to reduce noise.
///
/// Uses a simple 3-point averaging filter, preserving endpoints.
/// This reduces phase noise while maintaining the overall phase trajectory.
fn smooth_phase<F>(phases: &[F]) -> Vec<F>
where
    F: num_traits::Float + Copy,
{
    if phases.len() < 3 {
        return phases.to_vec();
    }

    let mut smoothed = Vec::with_capacity(phases.len());
    smoothed.push(phases[0]); // Keep first value unchanged

    // Apply 3-point smoothing to interior points
    for i in 1..phases.len() - 1 {
        let smoothed_phase = (phases[i - 1] + F::from(2.0).unwrap() * phases[i] + phases[i + 1])
            / F::from(4.0).unwrap();
        smoothed.push(smoothed_phase);
    }

    smoothed.push(phases[phases.len() - 1]); // Keep last value unchanged
    smoothed
}

impl<F> FrequencyTrack<F>
where
    F: num_traits::Float + Copy + PartialOrd + num_traits::FromPrimitive + Default,
{
    /// Creates a new frequency track starting with the given frequency.
    pub fn new(initial_frequency: F) -> Self {
        Self {
            frames: Vec::new(),
            confidence: F::one(),
            last_frequency: initial_frequency,
        }
    }

    /// Adds a frame of data to this track, updating confidence based on frequency stability.
    pub fn add_frame(&mut self, frame_index: usize, frequency: F, magnitude: F, phase: F) {
        self.frames.resize(frame_index + 1, None);

        // Update confidence based on frequency stability
        if !self.frames.is_empty() {
            let frequency_deviation = (frequency - self.last_frequency).abs();
            let relative_deviation = frequency_deviation / self.last_frequency.max(F::one());
            let confidence_decay = F::from(0.1).unwrap();
            self.confidence = self.confidence
                * (F::one() - relative_deviation * confidence_decay).max(confidence_decay);
        }

        self.frames[frame_index] = Some(FrameData {
            frequency,
            magnitude,
            phase,
        });

        self.last_frequency = frequency;
    }

    /// Extends the track to the given frame index, filling gaps with None.
    pub fn fill_gaps_to_frame(&mut self, frame_index: usize) {
        self.frames.resize(frame_index + 1, None);
    }

    /// Checks if this track is valid for synthesis based on length and confidence.
    pub fn is_valid(&self, min_length: usize) -> bool {
        let active_frame_count = self.frames.iter().filter(|frame| frame.is_some()).count();
        active_frame_count >= min_length && self.confidence > F::from(0.1).unwrap()
    }

    /// Converts this frequency track to a Wave for synthesis.
    ///
    /// Creates amplitude and phase envelopes from the tracked data, applies
    /// appropriate scaling, and handles missing frames through interpolation.
    pub fn to_wave(&self, window_size: usize, sample_rate: F) -> Option<Wave<F>>
    where
        F: num_traits::Float + Copy + Default + num_traits::FromPrimitive,
    {
        let frame_data: Vec<&FrameData<F>> =
            self.frames.iter().filter_map(Option::as_ref).collect();

        if frame_data.is_empty() {
            return None;
        }

        // Calculate representative frequency using magnitude-weighted average
        let total_weight: F = frame_data
            .iter()
            .map(|frame| frame.magnitude)
            .fold(F::zero(), |acc, val| acc + val);
        if total_weight <= F::zero() {
            return None;
        }

        let weighted_frequency: F = frame_data
            .iter()
            .map(|frame| frame.frequency * frame.magnitude)
            .fold(F::zero(), |acc, val| acc + val)
            / total_weight;

        // Create amplitude and phase envelopes, interpolating missing frames
        let (amplitude_envelope, phase_envelope): (Vec<F>, Vec<F>) = self
            .frames
            .iter()
            .scan(F::zero(), |last_phase, frame_option| match frame_option {
                Some(data) => {
                    *last_phase = data.phase;
                    Some((data.magnitude, data.phase))
                }
                None => Some((F::zero(), *last_phase)), // Interpolate missing frames
            })
            .unzip();

        // Apply frequency-appropriate amplitude scaling
        let amplitude_scale = calculate_amplitude_scaling(
            &amplitude_envelope,
            window_size,
            sample_rate,
            weighted_frequency,
        );

        let normalized_amplitude: Vec<F> = amplitude_envelope
            .iter()
            .map(|&amplitude| amplitude * amplitude_scale * self.confidence)
            .collect();

        // Unwrap phase for continuity
        let unwrapped_phase = unwrap_phase(&phase_envelope);

        Some(Wave {
            sample_rate,
            frequency: weighted_frequency,
            amplitude: crate::Modulation::Envelope(normalized_amplitude),
            phase: crate::Modulation::Envelope(unwrapped_phase),
            func: crate::WaveFunc::Sine,
        })
    }
}
