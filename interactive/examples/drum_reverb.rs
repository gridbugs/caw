use caw_interactive::prelude::*;

fn snare(trigger: &Trigger, input: Input) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.1;
    let noise = noise().filter(compress().ratio(0.1).scale(100.0).build());
    let env = adsr_linear_01(&clock)
        .release_s(duration_s * 1.0)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    let noise =
        noise.filter(low_pass_moog_ladder(8000.0).resonance(2.0).build());
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 240;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz)
        .reset_trigger(trigger)
        .build();
    let dry = (noise + osc)
        .filter(down_sample(4.0).build())
        .mul_lazy(&env);
    let reverb = dry.filter(
        reverb()
            .damping(input.mouse.x_01())
            .room_size(input.mouse.y_01())
            .build(),
    );
    dry + reverb
}

fn voice(input: Input) -> Sf64 {
    snare(&input.keyboard.n1.to_trigger_rising_edge(), input)
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = voice(window.input());
    window.play(signal)
}
