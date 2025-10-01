use hound::{WavReader, WavSpec};
use std::f32::consts::PI;
use waver::{
    analysis,
    analysis::Spectrum,
    quantization::{dequantize_samples, QuantizeIterator},
    Waveform,
};

fn main() {
    // Generate a sample WAV file to analyze.
    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create("sine_440hz.wav", spec).unwrap();

    // Generate float samples and quantize to i16 for WAV output using iterator
    let float_samples = (0..44100)
        .map(|x| x as f32 / 44100.0)
        .map(|t| (t * 440.0 * 2.0 * PI).sin());

    for sample in float_samples.quantize::<i16>() {
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();

    // Read the generated WAV file.
    let mut reader = WavReader::open("sine_440hz.wav").unwrap();
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();

    // Convert integer samples back to floats for analysis using our new float-only API
    let float_samples_for_analysis: Vec<f32> = dequantize_samples(&samples);
    let waveform: Waveform<f32> =
        Waveform::from_recorded_samples(spec.sample_rate as f32, &float_samples_for_analysis);

    // Analyze the waveform.
    let spectrum: Spectrum<f32> = analysis::spectrum(&waveform, float_samples_for_analysis.len());

    // Find the dominant frequency.
    let (dominant_freq, _, _) = spectrum
        .data
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();

    println!("Dominant frequency: {dominant_freq} Hz");

    // Clean up the generated file.
    std::fs::remove_file("sine_440hz.wav").unwrap();
}
