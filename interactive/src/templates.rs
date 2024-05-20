use crate::input::{Input, Key};
use caw_core::prelude::*;
use std::cell::RefCell;

pub fn note_by_key_sequence(start_note: Note, keys: Vec<Key>) -> Vec<(Key, Note)> {
    keys.into_iter()
        .enumerate()
        .map(|(i, key)| (key, start_note.add_semitones(i as i16)))
        .collect()
}

fn opinionated_note_by_key(start_note: Note) -> Vec<(Key, Note)> {
    use Key::*;
    let top_row_base = start_note.add_octaves(1);
    let top_row = vec![
        Q,
        W,
        N3,
        E,
        N4,
        R,
        T,
        N6,
        Y,
        N7,
        U,
        N8,
        I,
        O,
        N0,
        P,
        Minus,
        LeftBracket,
        RightBracket,
    ];
    let bottom_row = vec![Z, X, D, C, F, V, B, H, N, J, M, K, Comma, Period];
    top_row
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, top_row_base.add_semitones(i as i16)))
        .chain(
            bottom_row
                .into_iter()
                .enumerate()
                .map(|(i, key)| (key, start_note.add_semitones(i as i16))),
        )
        .collect::<Vec<_>>()
}

pub fn keyboard_key_events(
    input: Input,
    notes_by_key: Vec<(Key, Note)>,
    velocity_01: impl Into<Sf64>,
) -> Signal<Vec<KeyEvent>> {
    let velocity_01 = velocity_01.into();
    let state = RefCell::new(
        notes_by_key
            .into_iter()
            .map(|(key, note)| (key, note, false))
            .collect::<Vec<_>>(),
    );
    Signal::from_fn(move |ctx| {
        let mut state = state.borrow_mut();
        let mut ret = Vec::new();
        for (key, note, pressed) in state.iter_mut() {
            let current = input.key(*key).sample(ctx);
            if current != *pressed {
                *pressed = current;
                ret.push(KeyEvent {
                    note: *note,
                    pressed: *pressed,
                    velocity_01: velocity_01.sample(ctx),
                });
            }
        }
        ret
    })
}

pub fn opinionated_key_events(
    input: Input,
    start_note: Note,
    velocity_01: impl Into<Sf64>,
) -> Signal<Vec<KeyEvent>> {
    keyboard_key_events(input, opinionated_note_by_key(start_note), velocity_01)
}
