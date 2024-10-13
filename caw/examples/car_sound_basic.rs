use caw::prelude::*;

fn run(signal: Sf64) -> anyhow::Result<()> {
    let mut signal_player = SignalPlayer::new()?;
    signal_player.play_sample_forever(signal);
}

fn make_voice(offset: f64) -> Sf64 {
    let gate = periodic_gate_s(
        oscillator_s(Waveform::Sine, 4.0)
            .reset_offset_01(0.5)
            .build()
            .signed_to_01()
            * 0.3
            + 0.08,
    )
    .duty_01(0.05)
    .offset_01(offset)
    .build();
    let freq_hz = 80.0;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz).build() + noise() * 0.8;
    let env = adsr_linear_01(&gate).release_s(0.1).build().exp_01(5.0);
    let volume_env = adsr_linear_01(gate).release_s(0.1).build();
    let signal = osc
        .filter(low_pass_moog_ladder(env * 5000.0).resonance(1.5).build())
        .mul_lazy(&volume_env);
    signal
}

fn main() -> anyhow::Result<()> {
    let signal =
        make_voice(0.0) + make_voice(0.3) + make_voice(0.6) + make_voice(0.8);
    run(signal)
}
