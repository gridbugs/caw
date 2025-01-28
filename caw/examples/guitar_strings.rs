use caw::prelude::*;

fn note_from_key(
    note_by_key: Vec<(KeyFrameSig, Note)>,
) -> FrameSig<impl FrameSigT<Item = Option<Note>>> {
    FrameSig::option_first_some(
        note_by_key
            .into_iter()
            .map(|(key, note)| key.on(move || note)),
    )
}

fn main() {
    let window = Window::builder().build();
    let input = window.input();
    let sig = {
        let keyboard = input.keyboard.clone();
        let (gate, note) = note_from_key(vec![
            (keyboard.q, Note::E2),
            (keyboard.w, Note::A2),
            (keyboard.e, Note::D3),
            (keyboard.r, Note::G3),
            (keyboard.t, Note::B3),
            (keyboard.y, Note::E4),
        ])
        .map(|note_opt| (note_opt.is_some(), note_opt))
        .unzip();
        let note = note.option_or_last(note::C4);
        let env = adsr_linear_01(gate).release_s(1.0).build().exp_01(1.0);
        oscillator(Saw, note.freq_hz())
            .build()
            .filter(low_pass::default(env * 10000.0))
            .filter(reverb::default())
    };
    window.play_mono(sig, Default::default()).unwrap();
}
