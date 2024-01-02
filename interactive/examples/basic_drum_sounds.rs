use currawong_interactive::prelude::*;

fn snare(trigger: &Trigger, input: Input) -> Sf64 {
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
    let noise = noise.filter(
        low_pass_moog_ladder(20000.0 * input.mouse.x_01())
            .resonance(2.0)
            .build(),
    );
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 240;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz)
        .reset_trigger(trigger)
        .build();
    (noise + osc)
        .filter(down_sample(input.mouse.y_01() * 20.0).build())
        .mul_lazy(&env)
}

fn voice(trigger: Trigger, input: Input) -> Sf64 {
    snare(&trigger, input)
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = voice(periodic_trigger_hz(4.0).build(), window.input());
    window.play(signal)
}
