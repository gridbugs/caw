use currawong_interactive::prelude::*;
use std::{cell::RefCell, rc::Rc};

fn key_to_chord(input: &Input) -> Signal<Vec<Note>> {
    use note_name::*;
    use AbstractChord as AC;
    let octave_base = NoteName::C.in_octave(4);
    let chord = move |ac: AbstractChord| ac.notes_in_octave(octave_base);
    first_some([
        input.key(Key::Z).on(move || chord(AC::major(C))),
        input.key(Key::X).on(move || chord(AC::minor(D))),
        input.key(Key::S).on(move || chord(AC::minor(C))),
        input.key(Key::C).on(move || chord(AC::minor(E))),
        input.key(Key::D).on(move || chord(AC::major(D))),
        input.key(Key::F).on(move || chord(AC::major(E))),
        input.key(Key::V).on(move || chord(AC::major(F))),
        input.key(Key::G).on(move || chord(AC::minor(F))),
        input.key(Key::B).on(move || chord(AC::major(G))),
        input.key(Key::H).on(move || chord(AC::minor(G))),
        input.key(Key::N).on(move || chord(AC::minor(A))),
        input.key(Key::J).on(move || chord(AC::major(A))),
        input.key(Key::M).on(move || chord(AC::diminished(B))),
        input.key(Key::Q).on(move || chord(AC::sus2(C))),
        input.key(Key::W).on(move || chord(AC::sus2(D))),
        input.key(Key::E).on(move || chord(AC::sus2(E))),
        input.key(Key::R).on(move || chord(AC::sus2(F))),
        input.key(Key::T).on(move || chord(AC::sus2(G))),
        input.key(Key::Y).on(move || chord(AC::sus2(A))),
        input.key(Key::U).on(move || chord(AC::sus2(B))),
        input.key(Key::N1).on(move || chord(AC::sus4(C))),
        input.key(Key::N2).on(move || chord(AC::sus4(D))),
        input.key(Key::N3).on(move || chord(AC::sus4(E))),
        input.key(Key::N4).on(move || chord(AC::sus4(F))),
        input.key(Key::N5).on(move || chord(AC::sus4(G))),
        input.key(Key::N6).on(move || chord(AC::sus4(A))),
        input.key(Key::N7).on(move || chord(AC::sus4(B))),
    ])
    .map(|opt| opt.unwrap_or_default())
}

fn chords_to_voice_descs(
    chords: Signal<Vec<Note>>,
    arp_on: bool,
    arp_s: Sf64,
    max: usize,
) -> Vec<(Sfreq, Gate)> {
    let state = Rc::new(RefCell::new(
        (0..max).map(|_| (0.0, false)).collect::<Vec<_>>(),
    ));
    let arp_trigger = periodic_trigger_s(arp_s).build();
    let arp_state = Rc::new(RefCell::new(0usize));
    let update = chords.map_ctx({
        let state = Rc::clone(&state);
        let arp_state = Rc::clone(&arp_state);
        move |notes, ctx| {
            let mut state = state.borrow_mut();
            let mut arp_state = arp_state.borrow_mut();
            for (i, state) in state.iter_mut().enumerate() {
                if let Some(note) = notes.get(i) {
                    let gate_state = !arp_on || *arp_state % notes.len() == i;
                    *state = (note.freq().hz(), gate_state);
                } else {
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
            let (hz, gate_raw) = update
                .map({
                    let state = Rc::clone(&state);
                    move |()| {
                        let state = state.borrow();
                        state[i]
                    }
                })
                .unzip();
            (sfreq_hz(hz), gate_raw.to_gate())
        })
        .collect::<Vec<_>>()
}

fn synth(freq: Sfreq, gate: Gate, input: &Input) -> Sf64 {
    let freq_hz = freq.hz();
    let osc = mean([
        oscillator_hz(Waveform::Saw, &freq_hz).build(),
        oscillator_hz(Waveform::Saw, &freq_hz * 1.01).build(),
        oscillator_hz(Waveform::Saw, &freq_hz * 0.99).build(),
    ]);
    let env = adsr_linear_01(&gate).attack_s(0.0).release_s(0.0).build();
    osc.filter(
        low_pass_moog_ladder(&env * 6000.0 * input.mouse.x_01())
            .resonance(0.0)
            .build(),
    )
}

fn voice(input: Input) -> Sf64 {
    let voice_descs = chords_to_voice_descs(key_to_chord(&input), false, input.mouse.y_01(), 4);
    let synths = voice_descs
        .iter()
        .map(|(freq, gate)| synth(freq.clone(), gate.clone(), &input))
        .collect::<Vec<_>>();
    sum(synths)
        .mix(|dry| {
            4.0 * input.mouse.y_01() * dry.filter(reverb().room_size(0.9).damping(0.1).build())
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
