use caw::prelude::*;

fn run(signal: Sf64) -> anyhow::Result<()> {
    let mut signal_player = SignalPlayer::new()?;
    signal_player.play_sample_forever(signal);
}

fn main() -> anyhow::Result<()> {
    let gate =
        periodic_gate_s(oscillator_s(Waveform::Sine, 10.0).build().signed_to_01() * 1.0 + 0.1)
            .duty_01(0.8)
            .build();
    let freq_hz = 200.0;
    let lfo_speed = oscillator_s(Waveform::Saw, 7.0).build().signed_to_01();
    let osc = oscillator_hz(Waveform::Pulse, freq_hz)
        .pulse_width_01(
            0.2 + oscillator_hz(Waveform::Sine, freq_hz / (15.0 * lfo_speed + 1.0))
                .build()
                .signed_to_01()
                * 0.3,
        )
        .build();
    let env = adsr_linear_01(&gate)
        .attack_s(0.1)
        .decay_s(1.0)
        .sustain_01(0.6)
        .release_s(0.2)
        .build()
        .exp_01(4.0);
    let volume_env = adsr_linear_01(gate).release_s(0.1).build();
    let signal = osc.filter(low_pass_moog_ladder(env * 5000.0).resonance(5.0).build()) * volume_env;
    run(signal)
}
