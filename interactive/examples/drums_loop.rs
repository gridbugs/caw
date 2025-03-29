use caw_core::*;
use caw_interactive::{Input, Visualization, Window};
use caw_modules::*;
use caw_patches::drum;
use caw_utils::bitwise_pattern_trigs_8;

fn sig(input: Input, ch: Channel) -> Sig<impl SigT<Item = f32>> {
    let phase_offset_01 = match ch {
        Channel::Left => 0.,
        Channel::Right => 0.25,
    };
    let kick = 1;
    let snare = 2;
    let hat_closed = 4;
    let [trig_kick, trig_snare, trig_hat_closed, ..] = bitwise_pattern_trigs_8(
        periodic_gate_s(input.mouse.x_01() + 0.01).build(),
        vec![
            kick,
            hat_closed,
            snare,
            hat_closed,
            kick | snare,
            kick,
            snare,
            hat_closed,
        ],
    );
    (trig_kick.trig(drum::kick().phase_offset_01(phase_offset_01))
        + trig_snare.trig(drum::snare().phase_offset_01(phase_offset_01))
        + trig_hat_closed.trig(drum::hat_closed()))
    .filter(low_pass::default(input.mouse.y_01() * 20_000.))
    .filter(reverb::default())
    .filter(high_pass::default(1.))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .scale(0.9)
        .build();
    let input = window.input();
    window.play_stereo(
        Stereo::new_fn_channel(|ch| sig(input.clone(), ch)),
        Default::default(),
    )
}
