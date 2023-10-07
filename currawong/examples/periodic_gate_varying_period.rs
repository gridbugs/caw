use currawong::prelude::*;

fn run(signal: Sf64) -> anyhow::Result<()> {
    let mut signal_player = SignalPlayer::new()?;
    signal_player.play_sample_forever(signal);
}

fn make_voice(freq_hz: f64, period_s: f64) -> Sf64 {
    let gate = periodic_gate_s(
        oscillator_s(Waveform::Sine, period_s)
            .build()
            .signed_to_01(),
    )
    .duty_01(0.5)
    .build();
    let osc = mean([
        oscillator_hz(Waveform::Pulse, freq_hz / 1.5).build(),
        oscillator_hz(Waveform::Saw, freq_hz).build(),
        oscillator_hz(Waveform::Saw, freq_hz * 1.01).build(),
        noise() * 2.0,
    ]);
    let env = adsr_linear_01(gate)
        .attack_s(0.1)
        .decay_s(0.1)
        .sustain_01(0.5)
        .release_s(0.1)
        .build()
        .filter(low_pass_butterworth(100.0).build());
    osc.filter(low_pass_moog_ladder(2000 * &env).resonance(4.0).build()) * env
}

fn main() -> anyhow::Result<()> {
    let signal = mean([
        make_voice(100.0, 10.0),
        make_voice(200.0, 7.0),
        make_voice(150.0, 11.0),
        make_voice(300.0, 3.0),
    ]);
    run(signal)
}
