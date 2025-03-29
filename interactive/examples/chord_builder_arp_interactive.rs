use caw_core::*;
use caw_interactive::{Input, Key, Visualization, Window};
use caw_keyboard::{chord::*, *};
use caw_modules::*;

fn input_to_chords(
    input: Input,
) -> FrameSig<impl FrameSigT<Item = Option<Chord>>> {
    use note_name::*;
    let key = |key, note_name, chord_type: ChordType| {
        let mut x = [
            input
                .key(key)
                .on(move || chord(note_name, chord_type))
                .boxed(),
            (input.key(key) & input.key(Key::Comma))
                .on(move || chord(note_name, chord_type.infer_7()))
                .boxed(),
            (input.key(key) & input.key(Key::Period))
                .on(move || chord(note_name, SUS_2))
                .boxed(),
            (input.key(key) & input.key(Key::Slash))
                .on(move || chord(note_name, SUS_4))
                .boxed(),
            (input.key(key) & input.key(Key::Apostrophe))
                .on(move || chord(note_name, chord_type).octave_shift(1))
                .boxed(),
            (input.key(key)
                & input.key(Key::Comma)
                & input.key(Key::Apostrophe))
            .on(move || chord(note_name, chord_type.infer_7()).octave_shift(1))
            .boxed(),
            (input.key(key)
                & input.key(Key::Period)
                & input.key(Key::Apostrophe))
            .on(move || chord(note_name, SUS_2).octave_shift(1))
            .boxed(),
            (input.key(key)
                & input.key(Key::Slash)
                & input.key(Key::Apostrophe))
            .on(move || chord(note_name, SUS_4).octave_shift(1))
            .boxed(),
        ];
        x.reverse();
        x
    };
    FrameSig::option_first_some(
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

fn voice(
    MonoVoice {
        note,
        key_down_gate,
        key_press_trig,
        ..
    }: MonoVoice<impl FrameSigT<Item = KeyEvents>>,
) -> Sig<impl SigT<Item = f32>> {
    let env = adsr_linear_01(key_down_gate)
        .key_press_trig(key_press_trig)
        .attack_s(0.00)
        .decay_s(0.2)
        .sustain_01(0.8)
        .release_s(0.1)
        .build()
        .exp_01(1.);
    let osc = super_saw(note.freq_hz()).num_oscillators(32).build();
    osc.filter(low_pass::default(env * 5000.).resonance(0.5))
}

fn signal(input: Input) -> Sig<impl SigT<Item = f32>> {
    let inversion = input.x_01().map(|x| Inversion::InOctave {
        octave_base: Note::from_midi_index((x * 40.0 + 40.0) as u8),
    });
    let gate = periodic_gate_s(input.y_01()).build().shared();
    let config = ArpConfig::default()
        .with_shape(ArpShape::UpDown)
        .with_extend_octaves_low(1)
        .with_extend_octaves_high(1);
    input_to_chords(input.clone())
        .key_events(ChordVoiceConfig::default().with_inversion(inversion))
        .arp(gate.clone(), config)
        .poly_voices(12)
        .into_iter()
        .map(voice)
        .sum::<Sig<_>>()
        .filter(high_pass::default(1.))
        .filter(chorus())
        .filter(delay_trig(gate.divide(4)))
        .filter(reverb::default().room_size(0.9).damping(0.5))
        .filter(high_pass::default(1.))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .build();
    let input = window.input();
    window.play_stereo(
        Stereo::new_fn(|| signal(input.clone())),
        Default::default(),
    )
}
