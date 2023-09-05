use ibis_interactive::{
    filters::*,
    oscillator::{Oscillator, Waveform},
    signal::Sf64,
    window::{Rgb24, Window},
};

fn run(signal: Sf64) -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(2.0)
        .stable(true)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let mut signal = signal * 0.1;
    window.play(&mut signal)
}

fn main() -> anyhow::Result<()> {
    let signal = (Oscillator::builder_hz(Waveform::Saw, 40.0).build_signal()
        + Oscillator::builder_hz(Waveform::Saw, 40.1).build_signal()
        + Oscillator::builder_hz(Waveform::Pulse, 120.0).build_signal()
        + Oscillator::builder_hz(Waveform::Pulse, 120.2).build_signal()
        + Oscillator::builder_hz(Waveform::Saw, 40.0)
            .reset_offset_01(0.5)
            .build_signal())
    .filter(LowPassChebyshev::builder(1000.0).resonance(4.0).build())
    .filter(Saturate::builder().scale(2.0).min(-1.0).max(2.0).build());
    run(signal)
}
