use caw_builders::*;
use caw_core_next::*;
use caw_interactive_next::window::{Visualization, Window};
use rgb_int::Rgb24;

fn signal_left() -> SigBuf<impl Sig<Item = f32, Buf = Vec<f32>>> {
    let lfo = (const_(160.0).buffered()
        + (oscillator(waveform::Triangle, freq_hz(0.002))
            .reset_offset_01(const_(0.25))
            .build()
            .buffered())
            * const_(1.0).buffered())
    .map(|x| freq_hz(x));
    oscillator(waveform::Sine, lfo).build().buffered()
}

fn signal_right() -> SigBuf<impl Sig<Item = f32, Buf = Vec<f32>>> {
    let lfo = (const_(200.0).buffered()
        + (oscillator(waveform::Triangle, freq_hz(-0.0013))
            .reset_offset_01(const_(0.25))
            .build()
            .buffered())
            * const_(1.0).buffered())
    .map(|x| freq_hz(x));
    oscillator(waveform::Sine, lfo).build().buffered()
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
