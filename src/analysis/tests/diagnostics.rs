//! Diagnostic tests for parameter limits and algorithm performance boundaries.
//!
//! These tests are expensive and produce verbose output, so they are gated behind
//! the "diagnostics" feature flag. To run them:
//!
//! ```bash
//! # Run only diagnostic tests
//! cargo test --features diagnostics analysis::tests::diagnostics
//!
//! # Run all tests including diagnostics  
//! cargo test --features diagnostics analysis::tests
//!
//! # Run normal tests without diagnostics (default)
//! cargo test analysis::tests
//! ```

use super::*;

#[test]
fn test_parameter_limits_diagnostic() {
    println!("\n=== COMPREHENSIVE FREQUENCY BAND ANALYSIS ===");
    println!("Analyzing optimal parameters across the entire electromagnetic spectrum");
    println!("From sub-Hz geological signals to GHz electromagnetic waves\n");

    // Define frequency bands with order-of-magnitude coverage
    let frequency_bands = vec![
        // Sub-Hz: Geological, tidal, very slow oscillations
        ("Sub-Hz", vec![0.001, 0.01, 0.1], vec![10.0, 100.0, 1000.0]),
        // Infrasound: 1-20 Hz (earthquakes, atmospheric, ocean waves)
        ("Infrasound", vec![1.0, 5.0, 10.0], vec![100.0, 1000.0]),
        // Audio: 20Hz-20kHz (human hearing range)
        (
            "Audio Low",
            vec![20.0, 50.0, 100.0],
            vec![1000.0, 8000.0, 44100.0],
        ),
        (
            "Audio Mid",
            vec![200.0, 440.0, 1000.0],
            vec![8000.0, 44100.0, 96000.0],
        ),
        (
            "Audio High",
            vec![2000.0, 8000.0, 20000.0],
            vec![44100.0, 96000.0, 192000.0],
        ),
        // Ultrasound: 20kHz-1MHz (medical, industrial, sonar)
        (
            "Ultrasound Low",
            vec![20000.0, 40000.0, 100000.0],
            vec![200000.0, 500000.0, 1000000.0],
        ),
        (
            "Ultrasound High",
            vec![100000.0, 500000.0, 1000000.0],
            vec![2000000.0, 5000000.0, 10000000.0],
        ),
        // Radio Frequency: 1MHz-1GHz (AM/FM radio, cellular, WiFi)
        (
            "RF Low",
            vec![1000000.0, 10000000.0, 100000000.0],
            vec![10000000.0, 100000000.0, 1000000000.0],
        ),
        (
            "RF High",
            vec![100000000.0, 500000000.0, 1000000000.0],
            vec![1000000000.0, 5000000000.0, 10000000000.0],
        ),
    ];

    // Collect dynamic findings for summary report
    let mut band_summaries = Vec::new();
    let mut total_tests = 0;
    let mut successful_frequencies = 0;
    let mut failed_frequencies = 0;

    for (band_name, frequencies, sample_rates) in frequency_bands {
        println!("\n{:=<60}", "");
        println!("🔬 ANALYZING {} BAND", band_name.to_uppercase());
        println!("{:=<60}", "");

        // Frequency-specific window sizes (optimize for each band)
        let window_sizes = if band_name.contains("RF") {
            vec![32, 64, 128, 256, 512] // RF doesn't need huge windows
        } else if band_name.contains("Ultrasound") {
            vec![64, 128, 256, 512, 1024] // Ultrasound moderate windows
        } else if band_name.contains("Audio") {
            vec![128, 256, 512, 1024, 2048] // Audio full range
        } else {
            vec![512, 1024, 2048, 4096, 8192] // Sub-Hz/Infrasound need large windows
        };

        let mut band_results = Vec::new();

        for &test_freq in &frequencies {
            total_tests += 1;
            // Format frequency with appropriate units
            let freq_display = if test_freq >= 1e9 {
                format!("{:.1} GHz", test_freq / 1e9)
            } else if test_freq >= 1e6 {
                format!("{:.1} MHz", test_freq / 1e6)
            } else if test_freq >= 1e3 {
                format!("{:.1} kHz", test_freq / 1e3)
            } else if test_freq >= 1.0 {
                format!("{:.1} Hz", test_freq)
            } else if test_freq >= 1e-3 {
                format!("{:.1} mHz", test_freq * 1e3)
            } else {
                format!("{:.4} Hz", test_freq)
            };

            println!("\n--- {} ({}) ---", freq_display, test_freq);

            let mut all_results = Vec::new();

            for &sample_rate in &sample_rates {
                // Skip if frequency exceeds Nyquist limit
                if test_freq >= sample_rate * 0.4 {
                    println!(
                        "  ⚠️  Skipping sample rate {:.0}Hz - frequency too close to Nyquist",
                        sample_rate
                    );
                    continue;
                }

                println!(
                    "  Testing with sample rate: {:.0} Hz (Nyquist: {:.0} Hz)",
                    sample_rate,
                    sample_rate * 0.5
                );

                for &window_size in &window_sizes {
                    // Frequency-adaptive sample count calculation
                    let target_cycles = if test_freq >= 1e6 {
                        3.0 // RF: minimal cycles (fast analysis)
                    } else if test_freq >= 1e3 {
                        5.0 // Audio/Ultrasound: moderate cycles
                    } else if test_freq >= 0.1 {
                        8.0 // Low audio/Infrasound: more cycles
                    } else if test_freq >= 0.01 {
                        5.0 // Very low frequencies: fewer cycles due to data constraints
                    } else {
                        3.0 // Extremely low frequencies: minimal cycles
                    };

                    // Calculate samples needed for target cycles
                    let samples_per_cycle = sample_rate / test_freq;
                    let target_samples = (target_cycles * samples_per_cycle) as usize;

                    // Ensure minimum samples for window analysis (at least 4 windows worth)
                    let min_samples = window_size * 4;
                    let num_samples = target_samples.max(min_samples);

                    // Cap maximum samples to control computational cost
                    let max_samples = if test_freq >= 1e6 {
                        50_000 // RF: keep very small for speed
                    } else if test_freq >= 1e3 {
                        500_000 // Audio: moderate size
                    } else if test_freq >= 0.1 {
                        2_000_000 // Infrasound: larger for accuracy
                    } else if test_freq >= 0.01 {
                        20_000_000 // Very low frequencies: allow more samples
                    } else {
                        50_000_000 // Extremely low frequencies: maximum samples for feasibility
                    };

                    let num_samples = num_samples.min(max_samples);

                    // Skip if still insufficient samples
                    if num_samples < window_size * 3 {
                        continue;
                    }

                    // Calculate actual metrics for this test
                    let actual_duration = num_samples as f32 / sample_rate;
                    let actual_cycles = test_freq * actual_duration;

                    // Skip if we can't get enough cycles even with max samples
                    if actual_cycles < 2.0 {
                        continue;
                    }

                    // Generate test signal
                    let samples: Vec<f32> = (0..num_samples)
                        .map(|i| {
                            let t = i as f32 / sample_rate;
                            (t * test_freq * 2.0 * std::f32::consts::PI).sin()
                        })
                        .collect();

                    let waveform: Waveform<f32> =
                        Waveform::from_recorded_samples(sample_rate, &samples);

                    // Test synthesis with current parameters
                    match synthesize(&waveform, window_size, window_size / 4, 5) {
                        Ok(synthesized) => {
                            if let WaveformSource::Generative(waves) = &synthesized.source {
                                if !waves.is_empty() {
                                    // Find best frequency match
                                    let closest_freq = waves
                                        .iter()
                                        .min_by(|a, b| {
                                            (a.frequency - test_freq)
                                                .abs()
                                                .partial_cmp(&(b.frequency - test_freq).abs())
                                                .unwrap()
                                        })
                                        .unwrap()
                                        .frequency;

                                    let error_percent = if test_freq > 0.0 {
                                        (closest_freq - test_freq).abs() / test_freq * 100.0
                                    } else {
                                        100.0
                                    };

                                    // Calculate diagnostic metrics
                                    let samples_per_cycle_actual = if test_freq > 0.0 {
                                        sample_rate / test_freq
                                    } else {
                                        f32::INFINITY
                                    };
                                    let frequency_resolution = sample_rate / window_size as f32;
                                    let resolution_ratio = if test_freq > 0.0 {
                                        frequency_resolution / test_freq
                                    } else {
                                        0.0
                                    };

                                    all_results.push((
                                        window_size,
                                        actual_duration,
                                        sample_rate,
                                        num_samples,
                                        error_percent,
                                        closest_freq,
                                        actual_cycles,
                                        samples_per_cycle_actual,
                                        frequency_resolution,
                                        resolution_ratio,
                                    ));
                                }
                            }
                        }
                        Err(_) => {
                            // Synthesis failed - parameters not suitable
                        }
                    }
                }
            }

            // Analyze results for this frequency
            if all_results.is_empty() {
                println!("  ❌ No successful synthesis found for any parameter combination!");
                failed_frequencies += 1;
                band_results.push((test_freq, None)); // Track failure
                continue;
            }

            successful_frequencies += 1;

            // Sort by error percentage
            all_results.sort_by(|a, b| a.4.partial_cmp(&b.4).unwrap());

            // Find optimal parameters for different error tolerances
            let error_thresholds = vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0];

            println!("  📊 OPTIMAL PARAMETERS by error tolerance:");
            for &threshold in &error_thresholds {
                if let Some(result) = all_results.iter().find(|r| r.4 <= threshold) {
                    let (
                        window,
                        duration,
                        sr,
                        samples,
                        error,
                        detected,
                        cycles,
                        samp_cycle,
                        freq_res,
                        res_ratio,
                    ) = *result;

                    println!("    ≤{:4.1}% error: window={:4}, duration={:7.4}s, sr={:10.0}Hz, samples={:8}",
                            threshold, window, duration, sr, samples);
                    println!("           → {:.3}% error ({} → {:.2}Hz), {:.1} cycles, {:.1} samp/cyc, res={:.1}Hz ({:.1}%)",
                            error, freq_display, detected, cycles, samp_cycle, freq_res, res_ratio * 100.0);

                    // Issue warnings for problematic conditions
                    if cycles < 3.0 {
                        println!(
                            "           ⚠️  LOW CYCLES: Only {:.1} cycles observed",
                            cycles
                        );
                    }
                    if samp_cycle < 8.0 && samp_cycle != f32::INFINITY {
                        println!(
                            "           ⚠️  ALIASING RISK: Only {:.1} samples per cycle",
                            samp_cycle
                        );
                    }
                    if res_ratio > 0.2 {
                        println!(
                            "           ⚠️  POOR RESOLUTION: Freq resolution is {:.1}% of target",
                            res_ratio * 100.0
                        );
                    }

                    break; // Found minimum viable for this threshold
                }
            }

            // Show absolute best performance
            if let Some(best) = all_results.first() {
                let (
                    window,
                    duration,
                    sr,
                    samples,
                    error,
                    detected,
                    cycles,
                    samp_cycle,
                    freq_res,
                    res_ratio,
                ) = *best;
                println!("  🎯 BEST ACHIEVED: {:.4}% error", error);
                println!(
                    "     Parameters: window={}, duration={:.4}s, sr={:.0}Hz, samples={}",
                    window, duration, sr, samples
                );
                println!("     Performance: {} → {:.4}Hz, {:.1} cycles, {:.1} samp/cyc, res={:.2}Hz ({:.2}%)",
                        freq_display, detected, cycles, samp_cycle, freq_res, res_ratio * 100.0);

                // Store result for summary
                band_results.push((test_freq, Some((error, window, samples, cycles))));
            }

            // Window size analysis
            let mut window_performance: std::collections::HashMap<usize, f32> =
                std::collections::HashMap::new();
            for result in &all_results {
                let (window, _, _, _, error, _, _, _, _, _) = *result;
                window_performance
                    .entry(window)
                    .and_modify(|e| *e = e.min(error))
                    .or_insert(error);
            }

            let mut sorted_windows: Vec<_> = window_performance.into_iter().collect();
            sorted_windows.sort_by_key(|&(size, _)| size);

            println!("  🪟 Window size performance (best error achieved):");
            for (size, min_error) in sorted_windows {
                let status = if min_error <= 1.0 {
                    "🟢"
                } else if min_error <= 5.0 {
                    "🟡"
                } else {
                    "🔴"
                };
                println!("     {} {:4} → {:6.2}%", status, size, min_error);
            }
        }

        // Store band summary
        band_summaries.push((band_name, band_results));
    }

    // Generate dynamic summary report
    println!("\n{:=<80}", "");
    println!("🎯 COMPREHENSIVE FREQUENCY ANALYSIS COMPLETE");
    println!("{:=<80}", "");
    println!();

    println!("�� ANALYSIS STATISTICS");
    println!("• Total frequencies tested: {}", total_tests);
    println!(
        "• Successful synthesis: {} ({:.1}%)",
        successful_frequencies,
        successful_frequencies as f32 / total_tests as f32 * 100.0
    );
    println!(
        "• Failed synthesis: {} ({:.1}%)",
        failed_frequencies,
        failed_frequencies as f32 / total_tests as f32 * 100.0
    );
    println!();

    println!("📋 DYNAMIC FINDINGS BY FREQUENCY BAND:");
    for (band_name, results) in &band_summaries {
        let successful_count = results.iter().filter(|(_, opt)| opt.is_some()).count();
        let total_count = results.len();

        println!(
            "• {}: {}/{} successful",
            band_name, successful_count, total_count
        );

        if successful_count > 0 {
            let successful_results: Vec<_> = results
                .iter()
                .filter_map(|(freq, opt)| opt.map(|data| (*freq, data)))
                .collect();

            let best_error = successful_results
                .iter()
                .map(|(_, (error, _, _, _))| *error)
                .fold(f32::INFINITY, f32::min);

            let avg_error = successful_results
                .iter()
                .map(|(_, (error, _, _, _))| *error)
                .sum::<f32>()
                / successful_results.len() as f32;

            let optimal_window = successful_results
                .iter()
                .min_by(|(_, (error_a, _, _, _)), (_, (error_b, _, _, _))| {
                    error_a.partial_cmp(error_b).unwrap()
                })
                .map(|(_, (_, window, _, _))| *window);

            let avg_samples = successful_results
                .iter()
                .map(|(_, (_, _, samples, _))| *samples as f32)
                .sum::<f32>()
                / successful_results.len() as f32;

            println!(
                "    → Best error: {:.3}%, Avg error: {:.1}%",
                best_error, avg_error
            );
            if let Some(window) = optimal_window {
                println!(
                    "    → Optimal window: {}, Avg samples: {:.0}",
                    window, avg_samples
                );
            }

            // Show failed frequencies if any
            let failed_freqs: Vec<_> = results
                .iter()
                .filter_map(|(freq, opt)| if opt.is_none() { Some(*freq) } else { None })
                .collect();
            if !failed_freqs.is_empty() {
                println!("    → Failed frequencies: {:?}", failed_freqs);
            }
        } else {
            println!("    → All frequencies failed - check parameter limits");
        }
    }

    println!();
    println!("⚙️  ADAPTIVE PARAMETER INSIGHTS:");

    // Find most challenging and easiest frequency bands
    let mut band_difficulty: Vec<_> = band_summaries
        .iter()
        .map(|(name, results)| {
            let success_rate = results.iter().filter(|(_, opt)| opt.is_some()).count() as f32
                / results.len() as f32;
            let avg_error = if success_rate > 0.0 {
                results
                    .iter()
                    .filter_map(|(_, opt)| opt.map(|(error, _, _, _)| error))
                    .sum::<f32>()
                    / (results.len() as f32 * success_rate)
            } else {
                f32::INFINITY
            };
            (name, success_rate, avg_error)
        })
        .collect();

    band_difficulty.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    if let Some((easiest_band, success_rate, avg_error)) = band_difficulty.first() {
        println!(
            "• Easiest band: {} ({:.0}% success, {:.1}% avg error)",
            easiest_band,
            success_rate * 100.0,
            avg_error
        );
    }

    if let Some((hardest_band, success_rate, avg_error)) = band_difficulty.last() {
        if *success_rate < 1.0 {
            println!(
                "• Most challenging: {} ({:.0}% success, {:.1}% avg error)",
                hardest_band,
                success_rate * 100.0,
                if avg_error.is_infinite() {
                    0.0
                } else {
                    *avg_error
                }
            );
        }
    }

    println!("• Sample count formula: (target_cycles × sample_rate) / frequency");
    println!("• Target cycles: Sub-Hz=3-5, Infrasound=8, Audio=5, Ultrasound=5, RF=3");
    println!("• Max samples: Sub-Hz=50M, Infrasound=2M, Audio=500k, RF=50k");

    println!();
    println!("🚨 CRITICAL FINDINGS:");
    if failed_frequencies > 0 {
        println!(
            "• {} frequencies failed synthesis - may need parameter adjustment",
            failed_frequencies
        );
    }

    let very_low_freq_failures = band_summaries
        .iter()
        .find(|(name, _)| name.contains("Sub-Hz"))
        .map(|(_, results)| results.iter().filter(|(_, opt)| opt.is_none()).count())
        .unwrap_or(0);

    if very_low_freq_failures > 0 {
        println!("• Sub-Hz frequencies require massive sample counts (millions)");
        println!("• Consider specialized long-duration analysis for <0.01 Hz signals");
    }

    println!("• Algorithm performance varies significantly across frequency spectrum");
    println!("• Higher frequencies enable smaller, faster analysis windows");
}
