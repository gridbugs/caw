use currawong::prelude::*;

fn run(signal: Sf64) -> anyhow::Result<()> {
    let mut signal_player = SignalPlayer::new()?;
    signal_player.play_sample_forever(signal);
}

fn main() -> anyhow::Result<()> {
    let gate = periodic_gate_s(
        (oscillator_s(Waveform::Saw, 2.0).build() * -1).signed_to_01() * 0.4 + 0.05,
    )
    .build();
    let sah = noise()
        .signed_to_01()
        .filter(sample_and_hold(periodic_gate_s(0.2).build().to_trigger_rising_edge()).build())
        .filter(low_pass_butterworth(10.0).build());
    let freq_hz = 10.0;
    let osc = oscillator_hz(Waveform::Triangle, freq_hz).build();
    let env = adsr_linear_01(&gate)
        .attack_s(0.01)
        .release_s(0.1)
        .build()
        .exp_01(5.0);
    let volume_env = adsr_linear_01(gate).attack_s(0.01).release_s(0.1).build();
    let signal = osc
        .filter(
            low_pass_moog_ladder(env * (1000.0 + 1500.0 * sah))
                .resonance(2.0)
                .build(),
        )
        .mul_lazy(&volume_env);
    run(signal)
}
