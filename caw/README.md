![CAW Logo](../assets/logo.png)

# caw

[![Version](https://img.shields.io/crates/v/caw.svg)](https://crates.io/crates/caw)
[![Documentation](https://docs.rs/caw/badge.svg)](https://docs.rs/caw)
[![test](https://github.com/gridbugs/caw/actions/workflows/test.yml/badge.svg)](https://github.com/gridbugs/caw/actions/workflows/test.yml)
[![dependency status](https://deps.rs/repo/github/gridbugs/caw/status.svg)](https://deps.rs/repo/github/gridbugs/caw)

CAW is a framework for building synthesizers as Rust programs.

Here's a simple example that plays a 60Hz saw wave:
```toml
# Cargo.toml

[dependencies]
caw = { version = "0.4", features = ["interactive"] }
```
```rust
use caw::prelude::*;

fn main() {
    // Open a window that can play and visualize an audio signal.
    let window = Window::builder().build();

    // Describe the audio signal.
    let sig = oscillator(Saw, 60.0).build();

    // Play the audio signal, visualizing its waveform in the window.
    window.play_mono(sig, Default::default());
}
```

Filters can be applied to audio signals to build up more complex sounds. In
this next example the mouse position within the window controls a low-pass
filter cutoff and resonance.

```rust
use caw::prelude::*;

fn main() {
    // Open a window that can play and visualize an audio signal.
    let window = Window::builder().build();


    // Describe the audio signal.
    let input = window.input();
    let sig = oscillator(Saw, 60.0).build().filter(
        low_pass::default(input.mouse.x_01() * 20_000.0)
            .resonance(input.mouse.y_01()),
    );

    // Play the audio signal, visualizing its waveform in the window.
    window.play_mono(sig, Default::default()).unwrap();
}
```

It's possible to treat your computer keyboard as a musical keyboard in a few
more lines of code.

```rust
use caw::prelude::*;

fn main() {
    // Open a window that can play and visualize an audio signal.
    let window = Window::builder().build();

    // Describe the audio signal.
    let input = window.input();
    let MonoVoice {
        note,
        key_down_gate,
        ..
    } = input.keyboard.opinionated_key_events(Note::B2).mono_voice();
    let env = adsr_linear_01(key_down_gate).attack_s(0.1).build();
    let sig = oscillator(Saw, note.freq_hz()).build().filter(
        low_pass::default(env * input.mouse.x_01() * 20_000.0)
            .resonance(input.mouse.y_01()),
    )
    .filter(chorus())
    .filter(reverb::default());

    // Play the audio signal, visualizing its waveform in the window.
    window.play_mono(sig, Default::default()).unwrap();
}
```

There's a bunch of effects that can be chained onto signals. Here we'll add a
chorus and reverb effect.

See the examples in the [caw](../caw) and [caw_interactive](../interactive) crates for
more complex examples.
