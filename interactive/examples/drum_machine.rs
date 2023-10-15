use currawong_interactive::prelude::*;

fn kick(trigger: Trigger) -> Sf64 {
    let gate = trigger.to_gate();
    let freq_hz = 80.0;
    let lfo = oscillator_hz(Waveform::Sine, freq_hz / 4.0)
        .build()
        .signed_to_01()
        * 0.4
        + 0.1;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz)
        .pulse_width_01(lfo)
        .build()
        + noise().filter(low_pass_butterworth(200.0).build());
    let env = adsr_linear_01(gate).release_s(0.4).build().exp_01(8.0);
    let res = 0.0;
    let voice = mean([osc.filter(low_pass_moog_ladder(4000 * &env).resonance(res).build())]);
    voice
}

fn snare(trigger: Trigger) -> Sf64 {
    let gate = trigger.to_gate();
    let osc = noise();
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_butterworth(100.0).build());
    let voice = mean([osc.filter(low_pass_moog_ladder(5000 * &env).resonance(1.0).build())]);
    voice * 0.5
}

fn cymbal(trigger: Trigger) -> Sf64 {
    let gate = trigger.to_gate();
    let osc = noise();
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_butterworth(100.0).build());
    osc.filter(low_pass_moog_ladder(8000 * &env).build())
        .filter(high_pass_butterworth(6000.0).build())
        * 0.3
}

fn voice(input: Input) -> Sf64 {
    let clock = periodic_trigger_hz(8.0).build();
    let mk_loop = |add, remove, length| {
        clocked_trigger_looper()
            .clock(&clock)
            .add(input.keyboard.get(add))
            .remove(input.keyboard.get(remove))
            .length(length)
            .build()
    };
    let kick_loop = kick(mk_loop(Key::Q, Key::A, 16));
    let snare_loop = snare(mk_loop(Key::W, Key::S, 8));
    let cymbal_loop = cymbal(mk_loop(Key::E, Key::D, 8));
    sum([kick_loop, snare_loop, cymbal_loop])
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(0.2)
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = voice(window.input());
    window.play(signal)
}
