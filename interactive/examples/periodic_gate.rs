use ibis_interactive::{
    clock::PeriodicGate,
    envelope::AdsrLinear01,
    filters::*,
    oscillator::{Oscillator, Waveform},
    signal::{freq_hz, freq_s, Sf64},
    window::{Rgb24, Window},
};

fn run(signal: Sf64) -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(2.0)
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
    let gate = PeriodicGate::builder(freq_s(0.2)).build_gate();
    let osc = Oscillator::builder(Waveform::Pulse, freq_hz(40.0))
        .pulse_width_01(0.1)
        .build_signal();
    let env = AdsrLinear01::builder(gate).release_s(0.1).build_signal();
    let signal = osc
        .filter(LowPassButterworth::builder(&env * 4000.0 + 200.0).build())
        .filter(Saturate::builder().scale(2.0).min(-1.0).max(2.0).build())
        * env;
    run(signal)
}
