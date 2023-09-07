use ibis_interactive::{
    clock::PeriodicGate,
    envelope::AdsrLinear01,
    filters::*,
    oscillator::{Oscillator, Waveform},
    signal::Sf64,
    window::{Rgb24, Window},
};

fn run(signal: Sf64) -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(5.0)
        .stable(false)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let mut signal = signal * 0.1;
    window.play(&mut signal)
}

fn main() -> anyhow::Result<()> {
    let gate = PeriodicGate::builder_s(0.2).duty_01(0.90).build_gate();
    let lfo = Oscillator::builder_s(Waveform::Sine, 16.0)
        .reset_offset_01(-0.25)
        .build_signal()
        .signed_to_01();
    let osc = Oscillator::builder_hz(Waveform::Saw, 120.0).build_signal();
    let env = AdsrLinear01::builder(gate.clone())
        .attack_s(0.0)
        .decay_s(0.3)
        .sustain_01(0.0)
        .release_s(0.0)
        .build_signal()
        .exp_01(5.0);
    let volume_env = AdsrLinear01::builder(gate).release_s(0.2).build_signal();
    let signal = osc
        .filter(
            LowPassMoogLadder::builder(&env * (lfo * 5000.0 + 500.0) + 100.0)
                .resonance(4.0)
                .build(),
        )
        .filter(Saturate::builder().scale(2.0).threshold(2.0).build())
        * volume_env;
    run(signal)
}
