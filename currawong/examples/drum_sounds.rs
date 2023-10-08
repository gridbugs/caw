use currawong::prelude::*;

fn run(signal: Sf64) -> anyhow::Result<()> {
    let mut signal_player = SignalPlayer::new()?;
    signal_player.play_sample_forever(signal);
}

fn kick() -> Sf64 {
    let gate = periodic_gate_s(1.0).duty_01(0.01).build();
    let freq_hz = 80.0;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz).build() + noise() * 1.5;
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_butterworth(100.0).build());
    let voice = mean([
        osc.filter(low_pass_moog_ladder(3000 * &env).build()) * 0.5,
        osc.filter(low_pass_moog_ladder(2000 * &env).build()),
        osc.filter(low_pass_moog_ladder(1000 * &env).build()),
        osc.filter(low_pass_moog_ladder(500 * &env).build()) * 2,
        osc.filter(low_pass_moog_ladder(250 * &env).build()) * 2,
    ]) * 4.0;
    voice * env
}

fn snare() -> Sf64 {
    let gate = periodic_gate_s(0.5).duty_01(0.01).build();
    let osc = noise();
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_butterworth(100.0).build());
    let voice = mean([osc.filter(low_pass_moog_ladder(5000 * &env).resonance(1.0).build())]);
    voice * env
}

fn cymbal() -> Sf64 {
    let gate = periodic_gate_s(0.25).duty_01(0.01).build();
    let osc = noise();
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_butterworth(100.0).build());
    osc.filter(low_pass_moog_ladder(10000 * &env).build())
        .filter(high_pass_butterworth(6000.0).build())
        * env
}

fn main() -> anyhow::Result<()> {
    let signal = sum([kick(), cymbal(), snare()]);
    run(signal)
}
