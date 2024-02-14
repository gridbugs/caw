use currawong_interactive::prelude::*;

fn voice(
    VoiceDesc {
        note,
        key_down,
        key_press,
        ..
    }: VoiceDesc,
    effect_x: Sf64,
    effect_y: Sf64,
) -> Sf64 {
    let oscillator = supersaw(note.freq()).build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.0)
        .decay_s(0.2)
        .sustain_01(0.5)
        .release_s(4.0)
        .build()
        .exp_01(1.0);
    oscillator.filter(
        low_pass_moog_ladder(env * (30 * note.freq_hz() * effect_x))
            .resonance(4.0 * effect_y)
            .build(),
    ) * 2.0
}

fn make_voice(input: Input) -> Sf64 {
    let arp_trigger = periodic_trigger_hz(8.0).build();
    opinionated_key_events(input.clone(), Note::C2, 1.0)
        .arpegiate(arp_trigger)
        .voice_descs_polyphonic(3, 5)
        .into_iter()
        .map(|voice_desc| voice(voice_desc, input.mouse.x_01(), input.mouse.y_01()))
        .sum::<Sf64>()
        .mix(|dry| dry.filter(reverb().room_size(1.0).damping(0.5).build()))
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
