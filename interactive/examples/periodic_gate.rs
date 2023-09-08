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
        .scale(2.0)
        .stable(true)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    window.play(signal * 0.5)
}

fn main() -> anyhow::Result<()> {
    let gate = PeriodicGate::builder_s(0.1).duty_01(0.10).build_gate();
    let lfo = Oscillator::builder_s(Waveform::Sine, 8.0)
        .reset_offset_01(-0.25)
        .build_signal()
        .signed_to_01();
    let osc = Oscillator::builder_hz(Waveform::Pulse, 120.0).build_signal();
    let env = AdsrLinear01::builder(gate.clone())
        .release_s(0.1)
        .build_signal()
        .exp_01(5.0);
    let volume_env = AdsrLinear01::builder(gate).release_s(0.1).build_signal();
    let signal = osc
        .filter(LowPassButterworth::builder(100.0).build())
        .filter(
            LowPassMoogLadder::builder(env * (lfo * 8000.0 + 500.0) + 100.0)
                .resonance(8.0)
                .build(),
        )
        .filter(HighPassButterworth::builder(100.0).build())
        * volume_env;
    run(signal)
}
