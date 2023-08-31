use ibis_interactive::{
    oscillator::{Oscillator, Waveform},
    window::{Rgb24, Window},
};

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .stable(false)
        .line_width(2)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let mut signal = Oscillator::builder(Waveform::Saw, 60.0).signal()
        + Oscillator::builder(Waveform::Saw, 60.5).signal()
        + Oscillator::builder(Waveform::Saw, 60.0)
            .reset_offset_01(0.5)
            .signal();
    window.play(&mut signal)
}
