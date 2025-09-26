use hound::{WavReader, WavSpec, WavWriter};
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
    let mut writer = WavWriter::create("original.wav", spec).unwrap();
    for t in (0..44100).map(|x| x as f32 / 44100.0) {
        let sample = (t * 440.0 * 2.0 * PI).sin() * 0.5 + (t * 880.0 * 2.0 * PI).sin() * 0.5;
        let amplitude = i16::MAX as f32;
        writer.write_sample((sample * amplitude) as i16).unwrap();
    }
    writer.finalize().unwrap();

    // Read the generated WAV file.
    let mut reader = WavReader::open("original.wav").unwrap();
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();

    // Create a waveform from the recorded samples.
    let waveform: Waveform<i16> =
        Waveform::from_recorded_samples(spec.sample_rate as f32, &samples);

    // Synthesize a new waveform.
    let synthesized_waveform = analysis::synthesize(&waveform, 1024, 256, 2).unwrap();

    // Write the synthesized waveform to a new WAV file.
    let mut writer = WavWriter::create("synthesized.wav", spec).unwrap();
    for sample in synthesized_waveform.iter() {
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();

    println!("Synthesized waveform saved to synthesized.wav");

    // Clean up the generated files.
    std::fs::remove_file("original.wav").unwrap();
    std::fs::remove_file("synthesized.wav").unwrap();
}
