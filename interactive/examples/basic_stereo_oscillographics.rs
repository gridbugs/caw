use caw_core::*;
use caw_interactive::{
    input::Input,
    window::{Visualization, Window},
};
use caw_modules::*;
use rgb_int::Rgb24;

fn signal_left(input: Input) -> Sig<impl SigT<Item = f32>> {
    let freq = 200.0 * input.mouse().y_01();
    oscillator(waveform::Sine, freq).build()
}

fn signal_right(input: Input) -> Sig<impl SigT<Item = f32>> {
    let freq = 300.0 * input.mouse().x_01();
    oscillator(waveform::Sine, freq)
        .reset_offset_01(0.25)
        .build()
}

fn run() -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(1.0)
        .stable(false)
        .line_width(1)
        .background(Rgb24::new(0, 0, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .visualization(Visualization::StereoOscillographics)
        .build();
    let input = window.input();
    window.play_stereo(
        Stereo::new(signal_left(input.clone()), signal_right(input.clone())),
        Default::default(),
    )
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    run()
}
