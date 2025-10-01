use hound::{WavSpec, WavWriter};
use waver::{Modulation, Wave, Waveform, quantization::QuantizeIterator};

fn main() {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    // --- Tremolo Example ---
    let carrier_wave = Wave {
        frequency: 440.0, // A4 note
        ..Default::default()
    };

    let tremolo_lfo = Wave {
        frequency: 5.0, // 5 Hz LFO for tremolo
        ..Default::default()
    };

    let tremolo_wave = Waveform::<f32>::with_wave(
        spec.sample_rate as f32,
        Wave {
            amplitude: Modulation::LFO(Box::new(tremolo_lfo)),
            ..carrier_wave.clone()
        },
    );

    let mut writer = WavWriter::create("tremolo.wav", spec).unwrap();
    // Generate float samples and quantize to i16 for WAV output using iterator
    for sample in tremolo_wave.iter().take(44100).quantize::<i16>() {
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();
    println!("Tremolo effect saved to tremolo.wav");

    // --- Vibrato Example ---
    let vibrato_lfo = Wave {
        frequency: 7.0, // 7 Hz LFO for vibrato
        ..Default::default()
    };

    let vibrato_wave = Waveform::<f32>::with_wave(
        spec.sample_rate as f32,
        Wave {
            phase: Modulation::LFO(Box::new(vibrato_lfo)),
            ..carrier_wave
        },
    );

    let mut writer = WavWriter::create("vibrato.wav", spec).unwrap();
    // Generate float samples and quantize to i16 for WAV output using iterator
    for sample in vibrato_wave.iter().take(44100).quantize::<i16>() {
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();
    println!("Vibrato effect saved to vibrato.wav");

    // Clean up the generated files
    std::fs::remove_file("tremolo.wav").unwrap();
    std::fs::remove_file("vibrato.wav").unwrap();
}
