use currawong_interactive::prelude::*;

fn voice(VoiceDesc { note, gate, .. }: VoiceDesc, effect_x: Sf64, effect_y: Sf64) -> Sf64 {
    pulse_pwm(note.freq())
        .offset_01(effect_x)
        .scale_01(effect_y)
        .reset_trigger(gate.to_trigger_rising_edge())
        .build()
        * gate.to_01()
}

fn make_voice(input: Input) -> Sf64 {
    voice(
        opinionated_key_events(input.clone(), Note::C1, 1.0).voice_desc_monophonic(),
        input.mouse.x_01(),
        input.mouse.y_01(),
    )
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(4.0)
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = make_voice(window.input());
    window.play(signal * 0.1)
}
