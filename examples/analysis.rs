use hound::{WavReader, WavSpec};
use std::f32::consts::PI;
use waver::{analysis, Waveform};

fn main() {
    // Generate a sample WAV file to analyze.
    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create("sine_440hz.wav", spec).unwrap();
    for t in (0..44100).map(|x| x as f32 / 44100.0) {
        let sample = (t * 440.0 * 2.0 * PI).sin();
        let amplitude = i16::MAX as f32;
        writer.write_sample((sample * amplitude) as i16).unwrap();
    }
    writer.finalize().unwrap();

    // Read the generated WAV file.
    let mut reader = WavReader::open("sine_440hz.wav").unwrap();
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();

    // Create a waveform from the recorded samples.
    let waveform: Waveform<i16> = Waveform::from_recorded_samples(spec.sample_rate as f32, &samples);

    // Analyze the waveform.
    let spectrum = analysis::spectrum(&waveform, samples.len());

    // Find the dominant frequency.
    let (dominant_freq, _) = spectrum
        .data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();

    println!("Dominant frequency: {} Hz", dominant_freq);

    // Clean up the generated file.
    std::fs::remove_file("sine_440hz.wav").unwrap();
}