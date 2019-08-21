Waver [![Build Status](https://img.shields.io/travis/amrali/waver/master.svg?logo=travis)](https://travis-ci.org/amrali/waver) [![codecov](https://img.shields.io/codecov/c/github/amrali/waver?logo=codecov)](https://codecov.io/gh/amrali/waver) [![Crates.io](https://img.shields.io/crates/v/waver.svg?logo=rust)](https://crates.io/crates/waver) [![Documentation](https://img.shields.io/badge/docs-current-blue.svg?logo=rust)](https://docs.rs/waver)
=====

A waveform generation library for Rust.

Waver is a simple no-std library to generate any waveform of a given frequency
and amplitude.

A waveform can be a simple sinusoidal wave or a complex waveform of varying
frequency and amplitude. Waver is useful where there's a need to generate
a simple sinusoidal sound wave or for constructing a frequency or amplitude
modulated carrier wave in bare-metal [Arduino] or a [Raspberry Pi].

## Usage

To use Waver, add the following to your `Cargo.toml` file.

```toml
[dependencies]
waver = "0.1"
```

## Example

```rust
use std::{vec::Vec, f32::consts::PI};
use waver::{Waveform, Wave};

fn main() {
  // 44.1Khz sampling rate and 16-bit depth.
  let mut wf = waver::Waveform::<i16>::new(44100.0);

  // Superpose a sine wave and a cosine wave.
  wf.superpose(Wave { frequency: 2600.0, ..Default::default() })
    .superpose(Wave { frequency: 2600.0, phase: PI / 2.0, ..Default::default() })
    .normalize_amplitudes();

  // Quantization of 100 samples
  let _output: Vec<i16> = wf.iter().take(100).collect();
}
```

## Features

* Arbitrary quantization levels. Specify the bit depth when constructing `Waveform`.
* Online wave generation. No buffers, infinite iterators.
* Wave superposition with weighted amplitudes.
* Modulate signal's frequency, amplitude or phase.
* Numerically stable, prevents clipping.

## TODO

* [ ] Implement checks to protect against aliasing (e.g., disallow frequencies above the Nyquist frequency).
* [ ] Use fixed-point arithmetic for platforms that doesn't have an FPU.
* [ ] Replace use of libm crate [when math support moves to libcore].
* [ ] Expand the crate features to also include signal analysis functionality.

## Contributing

Thought of something you'd like to see in Waver? You can visit the issue tracker
to check if it was reported or proposed before, and if not please feel free to
create an issue or feature request. Ready to start contributing?
The [contributing guide][contributing] is a good place to start. If you have
questions please feel free to ask.

[Arduino]: https://www.arduino.cc/
[Raspberry Pi]: https://www.raspberrypi.org/
[contributing]: https://github.com/amrali/waver/blob/master/CONTRIBUTING.md
[when math support moves to libcore]: https://github.com/rust-lang/rfcs/issues/2505
