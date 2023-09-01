use ibis_interactive::{
    filters::*,
    oscillator::{Oscillator, Waveform},
    window::{Rgb24, Window},
};

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(0.7)
        .stable(true)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let mut signal = (Oscillator::builder(Waveform::Saw, 40.0).signal()
        + Oscillator::builder(Waveform::Saw, 40.1).signal()
        + Oscillator::builder(Waveform::Pulse, 120.0).signal()
        + Oscillator::builder(Waveform::Pulse, 120.2).signal()
        + Oscillator::builder(Waveform::Saw, 40.0)
            .reset_offset_01(0.5)
            .signal())
    .filter(LowPassChebyshev::builder(1000.0).resonance(4.0).build())
    .filter(Saturate::builder().scale(2.0).min(-1.0).max(2.0).build());
    window.play(&mut signal)
}
