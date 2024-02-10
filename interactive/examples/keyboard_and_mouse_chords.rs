use currawong_interactive::prelude::*;
use std::{cell::RefCell, rc::Rc};

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
        ]
        .into_iter()
        .flatten(),
    )
}

fn chords_to_voice_descs(
    chords: Signal<Option<Chord>>,
    arp_on: bool,
    arp_s: Sf64,
    octave_base_input: Sf64,
    max: usize,
) -> Vec<(Sfreq, Gate, Sf64)> {
    let state = Rc::new(RefCell::new(
        (0..max).map(|_| (0.0, false, 0.0)).collect::<Vec<_>>(),
    ));
    let arp_trigger = periodic_trigger_s(arp_s).build();
    let arp_state = Rc::new(RefCell::new(0usize));
    let update = chords.map_ctx({
        let state = Rc::clone(&state);
        let arp_state = Rc::clone(&arp_state);
        move |chord, ctx| {
            let mut state = state.borrow_mut();
            let mut arp_state = arp_state.borrow_mut();
            if let Some(chord) = chord {
                let octave_base =
                    Note::from_midi_index((octave_base_input.sample(ctx) * 40.0 + 40.0) as u8);
                let notes = chord.notes_in_octave(octave_base);
                for (i, state) in state.iter_mut().enumerate() {
                    if let Some(note) = notes.get(i) {
                        let gate_state = !arp_on || *arp_state % notes.len() == i;
                        *state = (note.freq().hz(), gate_state, 1.0);
                    } else {
                        state.1 = false;
                    }
                }
            } else {
                for state in state.iter_mut() {
                    state.1 = false;
                }
            }
            if arp_trigger.sample(ctx) {
                *arp_state = arp_state.wrapping_add(1);
            }
        }
    });
    (0..max)
        .map(|i| {
            let (hz, gate_raw, vel) = update
                .map({
                    let state = Rc::clone(&state);
                    move |()| {
                        let state = state.borrow();
                        state[i]
                    }
                })
                .unzip();
            (sfreq_hz(hz), gate_raw.to_gate(), vel)
        })
        .collect::<Vec<_>>()
}

fn synth(freq: Sfreq, gate: Gate, vel: Sf64, _input: &Input) -> Sf64 {
    let osc = oscillator(Waveform::Saw, freq).build();
    let env = adsr_linear_01(&gate).attack_s(0.0).release_s(0.5).build();
    osc.filter(low_pass_moog_ladder(&env * 3000.0).resonance(0.0).build()) * vel
}

fn voice(input: Input) -> Sf64 {
    let voice_descs = chords_to_voice_descs(
        key_to_chord(&input),
        false,
        input.mouse.y_01(),
        input.mouse.x_01(),
        5,
    );
    let synths = voice_descs
        .iter()
        .map(|(freq, gate, vel)| synth(freq.clone(), gate.clone(), vel.clone(), &input))
        .collect::<Vec<_>>();
    sum(synths)
        .mix(|dry| 2.0 * dry.filter(reverb().room_size(0.9).damping(0.8).build()))
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
