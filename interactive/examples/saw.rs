use ibis_interactive::prelude::*;

fn run(signal: Sf64) -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(2.0)
        .stable(true)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    window.play(signal * 0.1)
}

fn main() -> anyhow::Result<()> {
    let signal = oscillator_hz(Waveform::Saw, 200.0).build();
    run(signal)
}
