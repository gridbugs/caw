use caw_core_next::*;
use caw_interactive_next::{
    input::Input,
    window::{Visualization, Window},
};
use caw_modules::*;
use caw_patches::pulse_pwm;
use rgb_int::Rgb24;

fn signal(input: Input, channel: Channel) -> Sig<impl SigT<Item = f32>> {
    pulse_pwm(input.mouse.x_01() * 100.)
        .lfo_freq_hz(input.mouse.y_01() * 4.0)
        .lfo_reset_offset_01(channel.circle_phase_offset_01())
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
        Stereo::new_fn_channel(|ch| signal(input.clone(), ch)),
        Default::default(),
    )
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    run()
}
