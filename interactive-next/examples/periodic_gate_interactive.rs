use caw_core_next::*;
use caw_interactive_next::*;
use caw_modules::*;

fn sig(input: Input) -> Sig<impl SigT<Item = f32>> {
    let trig = periodic_gate_s(input.mouse.x_01())
        .duty_01(input.mouse.y_01())
        .build();
    let osc = super_saw(60.0).build();
    let env = adsr_linear_01(trig).attack_s(0.1).release_s(0.1).build();
    osc.filter(low_pass::default(20_000. * env).resonance(0.5))
        .filter(reverb::default())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder().sane_default().build();
    window.play_mono(sig(window.input()), Default::default())
}
