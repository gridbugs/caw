use caw_builders::*;
use caw_core_next::*;
use caw_interactive_next::window::{Visualization, Window};
use rgb_int::Rgb24;

fn signal_left() -> Sig<impl SigT<Item = f32>> {
    let lfo = Sig(160.0)
        + (oscillator(waveform::Triangle, 0.002)
            .reset_offset_01(0.25)
            .build())
            * Sig(1.0);
    oscillator(waveform::Sine, lfo).build()
}

fn signal_right() -> Sig<impl SigT<Item = f32>> {
    let lfo = Sig(200.0)
        + (oscillator(waveform::Triangle, -0.0013)
            .reset_offset_01(0.25)
            .build())
            * Sig(1.0);
    oscillator(waveform::Sine, lfo).build()
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
    window.play_stereo(signal_left(), signal_right())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    run()
}
