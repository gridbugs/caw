use caw_interactive::prelude::*;

fn key_to_chord(input: &Input) -> Signal<Option<Chord>> {
    use note_name::*;
    let key = |key, note_name, chord_type: ChordType| {
        [
            input.key(key).on(move || chord(note_name, chord_type)),
            input
                .key(key)
                .and(input.key(Key::Comma))
                .on(move || chord(note_name, chord_type.infer_7())),
            input
                .key(key)
                .and(input.key(Key::Period))
                .on(move || chord(note_name, SUS_2)),
            input
                .key(key)
                .and(input.key(Key::Slash))
                .on(move || chord(note_name, SUS_4)),
            input
                .key(key)
                .and(input.key(Key::Apostrophe))
                .on(move || chord(note_name, chord_type).octave_shift(1)),
            input
                .key(key)
                .and(input.key(Key::Comma))
                .and(input.key(Key::Apostrophe))
                .on(move || {
                    chord(note_name, chord_type.infer_7()).octave_shift(1)
                }),
            input
                .key(key)
                .and(input.key(Key::Period))
                .and(input.key(Key::Apostrophe))
                .on(move || chord(note_name, SUS_2).octave_shift(1)),
            input
                .key(key)
                .and(input.key(Key::Slash))
                .and(input.key(Key::Apostrophe))
                .on(move || chord(note_name, SUS_4).octave_shift(1)),
        ]
    };
    first_some(
        [
            key(Key::Z, C, MAJOR),
            key(Key::X, D, MINOR),
            key(Key::C, E, MINOR),
            key(Key::V, F, MAJOR),
            key(Key::B, G, MAJOR),
            key(Key::N, A, MINOR),
            key(Key::M, B, DIMINISHED),
            key(Key::A, C, MINOR),
            key(Key::S, D, MAJOR),
            key(Key::D, E, MAJOR),
            key(Key::F, F, MINOR),
            key(Key::G, G, MINOR),
            key(Key::H, A, MAJOR),
            key(Key::J, B, MAJOR),
            key(Key::K, B, MINOR),
            key(Key::Q, C_SHARP, MAJOR),
            key(Key::W, C_SHARP, MINOR),
            key(Key::E, D_SHARP, MAJOR),
            key(Key::R, D_SHARP, MINOR),
            key(Key::T, F_SHARP, MAJOR),
            key(Key::Y, F_SHARP, MINOR),
            key(Key::U, G_SHARP, MAJOR),
            key(Key::I, G_SHARP, MINOR),
            key(Key::O, A_SHARP, MAJOR),
            key(Key::P, A_SHARP, MINOR),
        ]
        .into_iter()
        .flatten(),
    )
}

fn synth(
    VoiceDesc {
        note,
        key_down,
        key_press,
        ..
    }: VoiceDesc,
) -> Sf64 {
    let osc = oscillator(Waveform::Saw, note.freq()).build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.01)
        .release_s(0.01)
        .decay_s(0.5)
        .sustain_01(0.5)
        .build()
        .exp_01(1.0);
    osc.filter(low_pass_moog_ladder(&env * 2000.0).resonance(0.5).build())
}

fn voice(input: Input) -> Sf64 {
    let inversion = input.x_01().map(|x| Inversion::InOctave {
        octave_base: Note::from_midi_index((x * 40.0 + 40.0) as u8),
    });
    key_to_chord(&input)
        .key_events(ChordVoiceConfig::default().inversion(inversion))
        .polyphonic_with(5, 0, synth)
        .mix(|dry| {
            0.5 * dry.filter(reverb().room_size(0.5).damping(0.8).build())
        })
        .filter(high_pass_butterworth(20.0).build())
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
    let signal = voice(window.input());
    window.play(signal * 0.1)
}
