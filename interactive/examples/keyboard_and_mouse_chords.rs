use currawong_interactive::prelude::*;
use std::{cell::RefCell, rc::Rc};

fn key_to_chord(input: &Input) -> Signal<Option<AbstractChord>> {
    use note_name::*;
    use AbstractChord as AC;
    first_some([
        input.key(Key::Z).on(move || AC::major(C)),
        input.key(Key::S).on(move || AC::minor(C)),
        input.key(Key::X).on(move || AC::minor(D)),
        input.key(Key::D).on(move || AC::major(D)),
        input.key(Key::C).on(move || AC::minor(E)),
        input.key(Key::F).on(move || AC::major(E)),
        input.key(Key::V).on(move || AC::major(F)),
        input.key(Key::G).on(move || AC::minor(F)),
        input.key(Key::B).on(move || AC::major(G)),
        input.key(Key::H).on(move || AC::minor(G)),
        input.key(Key::N).on(move || AC::minor(A)),
        input.key(Key::J).on(move || AC::major(A)),
        input.key(Key::M).on(move || AC::diminished(B)),
        input.key(Key::Q).on(move || AC::sus2(C)),
        input.key(Key::W).on(move || AC::sus2(D)),
        input.key(Key::E).on(move || AC::sus2(E)),
        input.key(Key::R).on(move || AC::sus2(F)),
        input.key(Key::T).on(move || AC::sus2(G)),
        input.key(Key::Y).on(move || AC::sus2(A)),
        input.key(Key::U).on(move || AC::sus2(B)),
        input.key(Key::N1).on(move || AC::sus4(C)),
        input.key(Key::N2).on(move || AC::sus4(D)),
        input.key(Key::N3).on(move || AC::sus4(E)),
        input.key(Key::N4).on(move || AC::sus4(F)),
        input.key(Key::N5).on(move || AC::sus4(G)),
        input.key(Key::N6).on(move || AC::sus4(A)),
        input.key(Key::N7).on(move || AC::sus4(B)),
    ])
}

fn chords_to_voice_descs(
    chords: Signal<Option<AbstractChord>>,
    arp_on: bool,
    arp_s: Sf64,
    max: usize,
) -> Vec<(Sfreq, Gate, Sf64)> {
    use note_name::*;
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
                let notes = chord.notes_in_octave(C.in_octave(4));
                for (i, state) in state.iter_mut().enumerate() {
                    if let Some(note) = notes.get(i) {
                        let gate_state = !arp_on || *arp_state % notes.len() == i;
                        *state = (note.freq().hz(), gate_state, 1.0);
                    } else {
                        state.1 = false;
                    }
                }
                let bass_note = chord.root.in_octave(2);
                *state.last_mut().unwrap() = (bass_note.freq().hz(), true, 1.5);
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

fn synth(freq: Sfreq, gate: Gate, vel: Sf64, input: &Input) -> Sf64 {
    let osc = supersaw(freq).build();
    let env = adsr_linear_01(&gate).attack_s(0.0).release_s(0.0).build();
    osc.filter(
        low_pass_moog_ladder(&env * 6000.0 * input.mouse.x_01())
            .resonance(0.0)
            .build(),
    ) * vel
}

fn voice(input: Input) -> Sf64 {
    let voice_descs = chords_to_voice_descs(key_to_chord(&input), false, input.mouse.y_01(), 4);
    let synths = voice_descs
        .iter()
        .map(|(freq, gate, vel)| synth(freq.clone(), gate.clone(), vel.clone(), &input))
        .collect::<Vec<_>>();
    sum(synths)
        .mix(|dry| 2.0 * dry.filter(reverb().room_size(0.9).damping(0.1).build()))
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
