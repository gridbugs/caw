use caw_interactive::prelude::*;

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
    let oscillator = supersaw_hz(note.freq_hz()).build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.0)
        .decay_s(0.1)
        .sustain_01(0.1)
        .release_s(0.0)
        .build()
        .exp_01(1.0);
    oscillator.filter(
        low_pass_moog_ladder(env * (30 * note.freq_hz() * effect_x))
            .resonance(4.0 * effect_y)
            .build(),
    )
}

fn arp_shape(trigger: Trigger) -> Signal<ArpeggiatorShape> {
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use std::{cell::RefCell, rc::Rc};
    struct State {
        rng: StdRng,
        indices: Vec<Option<usize>>,
    }
    let state = Rc::new(RefCell::new(State {
        rng: StdRng::from_entropy(),
        indices: vec![Some(0); 8],
    }));
    const MAX_INDEX: usize = 6;
    trigger
        .divide(16)
        .on_unit({
            let state = Rc::clone(&state);
            move || {
                let mut state = state.borrow_mut();
                let index_to_change =
                    state.rng.gen::<usize>() % state.indices.len();
                let value = if state.indices.len() <= 1
                    || state.rng.gen::<f64>() < 0.9
                {
                    Some(state.rng.gen::<usize>() % MAX_INDEX)
                } else {
                    None
                };
                state.indices[index_to_change] = value;
            }
        })
        .map({
            let state = Rc::clone(&state);
            move |()| {
                let state = state.borrow();
                ArpeggiatorShape::Indices(state.indices.clone())
            }
        })
}

fn drum_loop(trigger: Trigger) -> Sf64 {
    let hat_closed = 1 << 0;
    let snare = 1 << 1;
    let kick = 1 << 2;
    #[rustfmt::skip]
    let pattern = vec![
        0,
        snare,
        0,
        kick,
        hat_closed,
        snare,
        0,
        kick,

    ];
    drum_loop_8(
        trigger.divide(1),
        pattern,
        vec![
            triggerable::hat_closed().build(),
            triggerable::snare().build(),
            triggerable::kick().build(),
        ],
    )
}

fn make_voice(input: Input) -> Sf64 {
    let arp_trigger = periodic_trigger_hz(8.0).build();
    let arp_config = ArpeggiatorConfig::default()
        .shape(arp_shape(arp_trigger.clone()))
        .extend_octaves_high(1);
    opinionated_key_events(input.clone(), Note::C2, 1.0)
        .arpeggiate(arp_trigger.clone(), arp_config)
        .voice_descs_polyphonic(1, 0)
        .into_iter()
        .map(|voice_desc| {
            voice(voice_desc, input.mouse.x_01(), input.mouse.y_01())
        })
        .sum::<Sf64>()
        .mix(|dry| dry.filter(reverb().room_size(1.0).damping(0.5).build()))
        .filter(high_pass_butterworth(1.0).build())
        + drum_loop(arp_trigger) * 0.5
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
