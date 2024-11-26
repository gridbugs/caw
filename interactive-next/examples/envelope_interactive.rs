use caw_core_next::*;
use caw_interactive_next::{Input, MouseButton, Visualization, Window};
use caw_modules::*;

fn signal(input: Input) -> Sig<impl SigT<Item = f32>> {
    let env = adsr_linear_01(input.mouse.button(MouseButton::Left))
        .attack_s(0.01)
        .decay_s(0.2)
        .sustain_01(0.8)
        .release_s(0.1)
        .build();
    let osc = super_saw(
        input.mouse.x_01().filter(
            sample_and_hold(input.mouse.button(MouseButton::Right))
                .initial_value(0.05),
        ) * 1000,
    )
    .build();
    osc.filter(
        low_pass::default(env * input.mouse.y_01() * 10000).resonance(0.5),
    )
    .filter(reverb::default())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .build();
    let input = window.input();
    window.play_stereo(
        signal(input.clone()),
        signal(input.clone()),
        Default::default(),
    )
}
