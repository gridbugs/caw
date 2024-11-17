use caw_builders::*;
use caw_core_next::*;
use caw_interactive_next::window::Window;
use rgb_int::Rgb24;

fn signal_left() -> SigBuf<impl Sig<Item = f32, Buf = Vec<f32>>> {
    oscillator(waveform::Saw, freq_hz(60.0)).build()
}

fn signal_right() -> SigBuf<impl Sig<Item = f32, Buf = Vec<f32>>> {
    oscillator(waveform::Saw, freq_hz(61.0)).build()
}

fn run() -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(2.0)
        .stable(true)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    window.play_stereo(signal_left(), signal_right())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    run()
}
