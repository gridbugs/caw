use caw_interactive::prelude::*;

fn run(signal: Sf64) -> anyhow::Result<()> {
    let window = Window::builder().build();
    window.play(signal * 0.1)
}

fn main() -> anyhow::Result<()> {
    let signal = (oscillator_hz(Waveform::Saw, 40.0).build()
        + oscillator_hz(Waveform::Saw, 40.1).build()
        + oscillator_hz(Waveform::Pulse, 120.0).build()
        + oscillator_hz(Waveform::Pulse, 120.2).build()
        + oscillator_hz(Waveform::Saw, 40.0)
            .reset_offset_01(0.5)
            .build())
    .filter(low_pass_chebyshev(1000.0).resonance(4.0).build())
    .filter(saturate().scale(2.0).min(-1.0).max(2.0).build());
    run(signal)
}
