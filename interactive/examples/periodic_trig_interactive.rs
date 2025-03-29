use caw_core::*;
use caw_interactive::*;
use caw_modules::*;

fn sig(input: Input) -> Sig<impl SigT<Item = f32>> {
    let trig = periodic_trig_s(input.mouse.x_01()).build();
    let osc = super_saw(40.0).build();
    let env = adsr_linear_01(trig).release_s(0.1).build();
    osc.filter(chorus())
        .filter(low_pass::default(20_000. * env).resonance(0.1))
        .filter(reverb::default())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder().sane_default().build();
    window.play_mono(sig(window.input()), Default::default())
}
