use caw::prelude::*;

pub fn melee(
    trig: impl FrameSigT<Item = bool> + 'static,
) -> Sig<impl SigT<Item = f32>> {
    FrameSig(trig)
        .trig(drum::kick())
        .filter(low_pass::moog_ladder::oberheim(4000.0))
        * 2.0
}

pub fn death(
    trig: impl FrameSigT<Item = bool> + 'static,
) -> Sig<impl SigT<Item = f32>> {
    let trig = FrameSig(trig).shared();
    let make_noise = || noise::white().filter(sample_and_hold(trig.clone()));
    let duration = 1.0;
    let env = adsr_linear_01(trig.clone())
        .key_press_trig(trig.clone().gate_to_trig_rising_edge())
        .release_s(duration)
        .build()
        .exp_01(1.0)
        .shared();
    let osc = oscillator(
        Pulse,
        (env.clone() * ((make_noise() * 100.) + 200.)) + 50.0,
    )
    .pulse_width_01(0.5)
    .build();
    let filtered_osc = osc
        .filter(down_sample(((1.0 - env.clone()) * 100.0) + 1.0))
        .filter(quantizer(10.0 * env.clone()));
    filtered_osc * 0.4 * env.map(|x| (x * 100.0).min(1.0))
}

pub fn pistol(
    trig: impl FrameSigT<Item = bool> + 'static,
) -> Sig<impl SigT<Item = f32>> {
    let trig = FrameSig(trig).shared();
    let make_noise = || noise::white().filter(sample_and_hold(trig.clone()));
    let duration = 0.2;
    let env = adsr_linear_01(trig.clone())
        .key_press_trig(trig.clone().gate_to_trig_rising_edge())
        .release_s(duration)
        .build()
        .exp_01(1.0)
        .shared();
    let osc = oscillator(
        Pulse,
        (env.clone() * ((make_noise() * 100.0) + 300.0)) + 20.0,
    )
    .pulse_width_01(0.1)
    .build();
    let filtered_osc = osc.filter(low_pass::moog_ladder::oberheim(
        env.clone() * (4000.0 + make_noise() * 2000.0),
    ));
    filtered_osc * env.clone()
}

pub fn shotgun(
    trig: impl FrameSigT<Item = bool> + 'static,
) -> Sig<impl SigT<Item = f32>> {
    let trig = FrameSig(trig).shared();
    let make_noise = || noise::white().filter(sample_and_hold(trig.clone()));
    let duration = 0.2;
    let env = adsr_linear_01(trig.clone())
        .key_press_trig(trig.clone().gate_to_trig_rising_edge())
        .release_s(duration)
        .build()
        .exp_01(1.0)
        .shared();
    let osc = oscillator(
        Sine,
        (env.clone() * ((make_noise() * 100.0) + 100.0)) + 20.0,
    )
    .build();
    let noise =
        noise::white().filter(low_pass::moog_ladder::oberheim(2000.0)) * 10.0;
    let filtered_osc = (osc + noise).filter(low_pass::moog_ladder::oberheim(
        env.clone() * (10000.0 + make_noise() * 5000.0),
    ));
    filtered_osc
}

pub fn rocket(
    trig: impl FrameSigT<Item = bool> + 'static,
) -> Sig<impl SigT<Item = f32>> {
    let trig = FrameSig(trig).gate_to_trig_rising_edge().shared();
    let make_noise = || noise::white().filter(sample_and_hold(trig.clone()));
    let duration = 0.4;
    let env = adsr_linear_01(trig.clone().trig_to_gate(duration))
        .key_press_trig(trig.clone())
        .attack_s(duration / 2.0)
        .release_s(duration * 2.0)
        .build()
        .exp_01(1.0)
        .shared();
    let noise =
        noise::white().filter(low_pass::moog_ladder::oberheim(8000.0)) * 10.0;
    let filtered_osc = noise.filter(low_pass::moog_ladder::oberheim(
        env.clone() * (5000.0 + make_noise() * 1000.0),
    ));
    filtered_osc
}

pub fn explosion(
    trig: impl FrameSigT<Item = bool> + 'static,
) -> Sig<impl SigT<Item = f32>> {
    let trig = FrameSig(trig).gate_to_trig_rising_edge().shared();
    let make_noise = || noise::white().filter(sample_and_hold(trig.clone()));
    let duration = 0.8;
    let env = ((adsr_linear_01(trig.clone().trig_to_gate(duration))
        .key_press_trig(trig.clone())
        .attack_s(duration / 4.0)
        .decay_s(3.0 * (duration / 4.0))
        .sustain_01(0.0)
        .build()
        .exp_01(1.0)
        + adsr_linear_01(trig.clone())
            .key_press_trig(trig.clone())
            .release_s(duration)
            .build()
            .exp_01(1.0))
        / 2.0)
        .shared();
    let osc = oscillator(
        Sine,
        (env.clone() * ((make_noise() * 100.0) + 100.0)) + 20.0,
    )
    .build();
    let noise =
        noise::white().filter(low_pass::moog_ladder::oberheim(2000.0)) * 10.0;
    let filtered_osc = (osc + noise).filter(low_pass::moog_ladder::oberheim(
        env.clone() * (10000.0 + make_noise() * 5000.0),
    ));
    filtered_osc
}

pub fn sig(input: Input) -> Sig<impl SigT<Item = f32>> {
    melee(input.keyboard.get(Key::N1).gate_to_trig_rising_edge())
        + death(input.keyboard.get(Key::N2).gate_to_trig_rising_edge())
        + pistol(input.keyboard.get(Key::N3).gate_to_trig_rising_edge())
        + shotgun(input.keyboard.get(Key::N4).gate_to_trig_rising_edge())
        + rocket(input.keyboard.get(Key::N5).gate_to_trig_rising_edge())
        + explosion(input.keyboard.get(Key::N6).gate_to_trig_rising_edge())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::Oscilloscope)
        .line_width(2)
        .scale(0.7)
        .stride(2)
        .build();
    let input = window.input();
    window.play_mono(sig(input), Default::default())
}
