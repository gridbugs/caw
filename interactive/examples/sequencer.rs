use caw_core::*;
use caw_interactive::{window::Window, Visualization};
use caw_modules::*;
use caw_utils::*;

fn sig() -> StereoPair<Sig<impl SigT<Item = f32>>> {
    let tempo_hz = svf32(1.);
    let clock = sv(periodic_trig_hz(tempo_hz.clone()).build());
    let seq = sv(value_sequencer(clock.clone(), [100., 200.]));
    Stereo::new_fn(|| {
        let freq_hz = seq.clone();
        let osc = super_saw(freq_hz).build();
        let env = adsr_linear_01(clock.clone()).r(0.1).build();
        osc.filter(low_pass::default(10_000. * env).q(0.))
            .filter(chorus())
    })
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(2.0)
        .visualization(Visualization::StereoOscillographics)
        .build();
    window.play_stereo(sig(), Default::default())
}
