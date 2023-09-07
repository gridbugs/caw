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
    let gate = PeriodicGate::builder_s(0.5).build_gate();
    let osc = Oscillator::builder_hz(Waveform::Saw, 80.0).build_signal();
    let env = AdsrLinear01::builder(gate.clone())
        .attack_s(0.01)
        .decay_s(0.2)
        .sustain_01(0.5)
        .release_s(0.01)
        .build_signal();
    let volume_env = AdsrLinear01::builder(gate).build_signal();
    let signal = osc
        .filter(
            LowPassChebyshev::builder(&env * 5000.0 + 1000.0)
                .resonance(40.0)
                .build(),
        )
        //.filter(HighPassChebyshev::builder(0.0).resonance(10.0).build())
        .filter(Saturate::builder().scale(1.0).threshold(2.0).build())
        * volume_env;
    run(signal)
}
