use currawong::prelude::*;

fn kick(trigger: &Trigger) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.1;
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 120;
    let osc = oscillator_hz(Waveform::Triangle, freq_hz).build();
    let env_amp = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    osc.mul_lazy(&env_amp)
        .filter(compress().ratio(0.02).scale(16.0).build())
}

fn snare(trigger: &Trigger) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.1;
    let noise = noise();
    let detune = 0.0002;
    let noise = (&noise
        + &noise.filter(delay_s(&detune))
        + &noise.filter(delay_s(&detune * 2.0))
        + &noise.filter(delay_s(&detune * 3.0)))
        .filter(compress().ratio(0.1).scale(100.0).build());
    let env = adsr_linear_01(&clock)
        .release_s(duration_s * 1.0)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    let noise = noise.filter(low_pass_moog_ladder(10000.0).resonance(2.0).build());
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 240;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz).build();
    (noise + osc).mul_lazy(&env)
}

fn cymbal(trigger: &Trigger) -> Sf64 {
    let gate = trigger.to_gate();
    let osc = noise();
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_moog_ladder(1000.0).build());
    osc.filter(low_pass_moog_ladder(10000 * &env).build())
        .filter(high_pass_butterworth(6000.0).build())
}
