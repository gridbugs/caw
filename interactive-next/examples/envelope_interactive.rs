use caw_core_next::*;
use caw_interactive_next::{Input, Key, Visualization, Window};
use caw_modules::*;

fn signal(input: Input) -> Sig<impl SigT<Item = f32>> {
    let env = adsr_linear_01(input.keyboard.get(Key::Space))
        .attack_s(0.5)
        .decay_s(0.2)
        .sustain_01(0.5)
        .release_s(2)
        .build();
    let osc = super_saw(input.mouse.x_01() * 1000).build() * 1.0;
    osc * env
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .build();
    let input = window.input();
    window.play_stereo(signal(input.clone()), signal(input.clone()))
}
