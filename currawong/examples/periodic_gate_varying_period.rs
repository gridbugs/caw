use currawong::prelude::*;
use currawong::signal_player::SignalPlayer;

fn run(signal: Sf64) -> anyhow::Result<()> {
    let mut signal_player = SignalPlayer::new()?;
    signal_player.play_sample_forever(signal);
}

fn main() -> anyhow::Result<()> {
    let gate = periodic_gate_s(0.1).duty_01(0.10).build();
    let lfo = oscillator_s(Waveform::Sine, 8.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01();
    let osc = oscillator_hz(Waveform::Pulse, 120.0).build();
    let env = adsr_linear_01(&gate).release_s(0.1).build().exp_01(5.0);
    let volume_env = adsr_linear_01(gate).release_s(0.1).build();
    let signal = osc
        .filter(low_pass_butterworth(100.0).build())
        .filter(
            low_pass_moog_ladder(env * (lfo * 8000.0 + 500.0) + 100.0)
                .resonance(8.0)
                .build(),
        )
        .filter(high_pass_butterworth(100.0).build())
        .mul_lazy(&volume_env);
    run(signal)
}
