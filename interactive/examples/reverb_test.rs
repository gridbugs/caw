use currawong_interactive::prelude::*;

fn voice(trigger: Trigger, input: Input) -> Sf64 {
    let gate = trigger.to_gate_with_duration_s(0.1);
    let osc = oscillator_hz(Waveform::Saw, 60.0).build();
    let env = adsr_linear_01(gate).build();
    let dry = osc * env;
    dry.filter(
        reverb()
            .damping(input.mouse.x_01())
            .room_size(input.mouse.y_01())
            .build(),
    ) + dry
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = voice(periodic_trigger_s(0.5).build(), window.input());
    window.play(signal)
}
