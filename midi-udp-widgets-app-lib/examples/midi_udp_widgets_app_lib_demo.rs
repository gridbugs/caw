use caw_core::*;
use caw_interactive::window::Window;
use caw_midi_udp_widgets_app_lib::{button, knob, xy};
use caw_modules::*;

fn sig() -> Sig<impl SigT<Item = f32>> {
    let env =
        adsr_linear_01(button("env gate").build().gate_to_trig_rising_edge())
            .release_s(0.5)
            .build();
    let (chorus_depth_01, chorus_lfo_rate_01) = xy("chorus").build().unzip();
    super_saw(60.)
        .build()
        .filter(
            low_pass::default(knob("low pass").build() * (env + 0.1) * 15_000.)
                .q(knob("q").build()),
        )
        .filter(
            chorus()
                .depth_s(chorus_depth_01 * 0.1)
                .lfo_rate_hz(chorus_lfo_rate_01 * 10.),
        )
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder().build();
    window.play_mono(sig(), Default::default())
}
