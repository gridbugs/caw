use caw_core_next::*;
use caw_interactive_next::{Input, Key, Visualization, Window};
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
    input: Input,
    MonoVoice {
        note,
        key_down_gate,
        key_press_trigger,
        ..
    }: MonoVoice<impl FrameSigT<Item = KeyEvents>>,
) -> Sig<impl SigT<Item = f32>> {
    let env = adsr_linear_01(key_down_gate)
        .key_press_trig(key_press_trigger)
        .attack_s(0.01)
        .decay_s(0.2)
        .sustain_01(0.8)
        .release_s(0.2)
        .build()
        .exp_01(1);
    let osc = super_saw(note.freq_hz()).num_oscillators(4).build();
    osc.filter(low_pass::default(env * 20000).resonance(0.5))
}

fn signal(input: Input) -> Sig<impl SigT<Item = f32>> {
    let inversion = input.x_01().map(|x| Inversion::InOctave {
        octave_base: Note::from_midi_index((x * 40.0 + 40.0) as u8),
    });
    let gate = periodic_gate_s(input.y_01()).build();
    let config = ArpConfig::default();
    let v = input_to_chords(input.clone())
        .key_events(ChordVoiceConfig::default().with_inversion(inversion))
        .arp(gate, config)
        .mono_voice();
    voice(input.clone(), v)
        .filter(reverb::default().room_size(0.9).damping(0.9))
        .filter(high_pass::default(1))
        * 0.25
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
