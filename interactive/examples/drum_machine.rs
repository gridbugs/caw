use currawong_interactive::prelude::*;

fn midi_index_by_gate() -> Vec<(Key, u8)> {
    use Key::*;
    let bottom_row_base = Note::new(NoteName::B, 1).to_midi_index();
    let bottom_row = vec![Z, X, D, C, F, V, B, H, N, J, M, K, Comma, Period];
    bottom_row
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, i as u8 + bottom_row_base))
        .collect::<Vec<_>>()
}

fn kick(trigger: Trigger) -> Sf64 {
    let gate = trigger.to_gate();
    let freq_hz = 30.0;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz)
        .pulse_width_01(0.5)
        .build();
    let env = adsr_linear_01(gate).release_s(0.2).build().exp_01(8.0);
    let voice = mean([osc.filter(low_pass_moog_ladder(4000 * &env).build())]);
    voice
}

fn snare(trigger: Trigger) -> Sf64 {
    let gate = trigger.to_gate();
    let osc = noise();
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_butterworth(100.0).build());
    let voice = mean([osc.filter(low_pass_moog_ladder(5000 * &env).resonance(1.0).build())]);
    voice * 0.5
}

fn cymbal(trigger: Trigger) -> Sf64 {
    let gate = trigger.to_gate();
    let osc = noise();
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_butterworth(100.0).build());
    osc.filter(low_pass_moog_ladder(10000 * &env).build())
        .filter(high_pass_butterworth(6000.0).build())
}

fn drum_looper(clock: &Trigger, input: &Input) -> Sf64 {
    let mk_loop = |add, remove, length| {
        clocked_trigger_looper()
            .clock(clock)
            .add(input.keyboard.get(add))
            .remove(input.keyboard.get(remove))
            .length(length)
            .build()
    };
    let kick_loop = kick(mk_loop(Key::Q, Key::N1, 16));
    let snare_loop = snare(mk_loop(Key::W, Key::N2, 16));
    let cymbal_loop = cymbal(mk_loop(Key::E, Key::N3, 16));
    sum([kick_loop, snare_loop, cymbal_loop])
}

fn synth_gate_and_midi_index(input: &Input) -> (Gate, Su8) {
    struct Entry {
        gate: Gate,
        last_pressed: u64,
        currently_pressed: bool,
        midi_index: u8,
    }
    let mut state = midi_index_by_gate()
        .into_iter()
        .map(|(key, midi_index)| {
            let gate = input.key(key);
            Entry {
                gate,
                last_pressed: 0,
                currently_pressed: false,
                midi_index,
            }
        })
        .collect::<Vec<_>>();
    let midi_index_and_gate_signal = Signal::from_fn(move |ctx| {
        let mut currently_pressed = false;
        let mut midi_index = 0;
        let mut newest_last_pressed = 0;
        let mut midi_index_all = 0;
        let mut newest_last_pressed_all = 0;
        for entry in state.iter_mut() {
            if entry.gate.sample(ctx) {
                currently_pressed = true;
                if !entry.currently_pressed {
                    entry.currently_pressed = true;
                    entry.last_pressed = ctx.sample_index;
                }
                if entry.last_pressed > newest_last_pressed {
                    newest_last_pressed = entry.last_pressed;
                    midi_index = entry.midi_index;
                }
            } else {
                entry.currently_pressed = false;
            }
            if entry.last_pressed > newest_last_pressed_all {
                newest_last_pressed_all = entry.last_pressed;
                midi_index_all = entry.midi_index;
            }
        }
        let freq_hz = if currently_pressed {
            midi_index
        } else {
            midi_index_all
        };
        (freq_hz, currently_pressed)
    });
    let midi_index = midi_index_and_gate_signal.map(|x| x.0);
    let gate = midi_index_and_gate_signal.map(|x| x.1).to_gate();
    (gate, midi_index)
}

fn synth_voice(gate: Gate, midi_index: Su8) -> Sf64 {
    let freq_hz = midi_index.midi_index_to_freq_hz_a440();
    let osc = oscillator_hz(Waveform::Saw, &freq_hz).build();
    let env = adsr_linear_01(gate)
        .decay_s(0.1)
        .sustain_01(0.0)
        .release_s(0.1)
        .build();
    let filtered_osc = osc.filter(low_pass_moog_ladder(env * 8000).resonance(2.0).build());
    filtered_osc * 0.5
}

fn synth_looper(clock: &Trigger, input: &Input) -> Sf64 {
    let (gate, midi_index) = synth_gate_and_midi_index(input);
    let (gate, midi_index) = clocked_midi_note_monophonic_looper()
        .clock(clock)
        .input_gate(gate)
        .input_midi_index(midi_index)
        .clear(&input.keyboard.slash)
        .length(16)
        .build();
    synth_voice(gate, midi_index)
}

fn voice(input: Input) -> Sf64 {
    let clock = periodic_trigger_hz(1.0 + &input.mouse.x_01 * 8.0).build();
    (drum_looper(&clock, &input) + synth_looper(&clock, &input)).filter(
        low_pass_moog_ladder(200.0 + input.mouse.y_01 * 10000.0)
            .resonance(1.0)
            .build(),
    )
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(0.2)
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = voice(window.input());
    window.play(signal)
}
