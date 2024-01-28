use crate::input::{Input, Key};
use currawong_core::prelude::*;
use std::cell::RefCell;

fn opinionated_freq_hz_by_key(start_note: Note) -> Vec<(Key, f64)> {
    use Key::*;
    let top_row_base = start_note.add_octaves(1).to_midi_index();
    let top_row = vec![
        Q,
        W,
        N3,
        E,
        N4,
        R,
        T,
        N5,
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
    let bottom_row_base = start_note.to_midi_index();
    let bottom_row = vec![Z, X, D, C, F, V, B, H, N, J, M, K, Comma, Period];
    top_row
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, freq_hz_of_midi_index(i as u8 + top_row_base)))
        .chain(
            bottom_row
                .into_iter()
                .enumerate()
                .map(|(i, key)| (key, freq_hz_of_midi_index(i as u8 + bottom_row_base))),
        )
        .collect::<Vec<_>>()
}

pub fn opinionated_key_events(
    input: Input,
    start_note: Note,
    velocity_01: impl Into<Sf64>,
) -> Signal<Vec<KeyEvent>> {
    let velocity_01 = velocity_01.into();
    let state = RefCell::new(
        opinionated_freq_hz_by_key(start_note)
            .into_iter()
            .map(|(key, freq_hz_)| (key, freq_hz(freq_hz_), false))
            .collect::<Vec<_>>(),
    );
    Signal::from_fn(move |ctx| {
        let mut state = state.borrow_mut();
        let mut ret = Vec::new();
        for (key, freq, pressed) in state.iter_mut() {
            let current = input.key(*key).sample(ctx);
            if current != *pressed {
                *pressed = current;
                ret.push(KeyEvent {
                    freq: *freq,
                    pressed: *pressed,
                    velocity_01: velocity_01.sample(ctx),
                });
            }
        }
        ret
    })
}
