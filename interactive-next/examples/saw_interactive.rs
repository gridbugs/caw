use caw_builders::*;
use caw_core_next::*;
use caw_interactive_next::window::Window;
use rgb_int::Rgb24;

fn signal() -> Sig<impl SigT<Item = f32>> {
    oscillator(waveform::Saw, 40.0)
        .build()
        .zip(oscillator(waveform::Saw, 40.1).build())
        .map(|(a, b)| (a + b) / 10.0)
}

fn run(signal: Sig<impl SigT<Item = f32>>) -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(2.0)
        .stable(true)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    window.play_mono(signal)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    run(signal())
}
