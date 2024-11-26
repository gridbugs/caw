use caw_interactive::prelude::*;

fn voice(
    VoiceDesc {
        note,
        key_down,
        key_press,
        ..
    }: VoiceDesc,
    _effect_x: Sf64,
    _effect_y: Sf64,
) -> Sf64 {
    let osc = oscillator_hz(Waveform::Saw, note.freq_hz() * 2).build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.0)
        .decay_s(0.8)
        .sustain_01(0.4)
        .release_s(0.5)
        .build()
        .exp_01(-1.0);
    
    osc.filter(
        low_pass_moog_ladder(env * (2000.0 + (1.0 * note.freq_hz()))).build(),
    ) * 2.0
}

fn make_voice(input: Input) -> Sf64 {
    keyboard_key_events(
        input.clone(),
        vec![
            (Key::N1, Note::E2),
            (Key::N2, Note::A2),
            (Key::N3, Note::D3),
            (Key::N4, Note::G3),
            (Key::N5, Note::B3),
            (Key::N6, Note::E4),
        ],
        1.0,
    )
    .voice_descs_polyphonic(3, 5)
    .into_iter()
    .map(|voice_desc| voice(voice_desc, input.mouse.x_01(), input.mouse.y_01()))
    .sum::<Sf64>()
    .mix(|dry| dry.filter(reverb().room_size(0.7).damping(0.3).build()))
    .filter(high_pass_butterworth(10.0).build())
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(1.0)
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = make_voice(window.input());
    window.play(signal * 0.1)
}
