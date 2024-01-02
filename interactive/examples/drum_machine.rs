use currawong_interactive::prelude::*;
use std::cell::RefCell;

fn midi_index_by_key() -> Vec<(Key, u8)> {
    use Key::*;
    let bottom_row_base = Note::new(NoteName::B, 1).to_midi_index();
    let bottom_row = vec![Z, X, D, C, F, V, B, H, N, J, M, K, Comma, Period];
    bottom_row
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, i as u8 + bottom_row_base))
        .collect::<Vec<_>>()
}

fn midi_index_by_key_bass() -> Vec<(Key, u8)> {
    use Key::*;
    let bottom_row_base = Note::new(NoteName::C, 2).to_midi_index();
    let bottom_row = vec![R, N5, T, N6, Y, U, N8, I, N9, O, N0, P, LeftBracket];
    bottom_row
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, i as u8 + bottom_row_base))
        .collect::<Vec<_>>()
}

fn kick(trigger: Trigger) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.1;
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 120;
    let osc = oscillator_hz(Waveform::Triangle, freq_hz).build();
    let env_amp = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    osc.mul_lazy(&env_amp)
        .filter(compress().ratio(0.02).scale(16.0).build())
}

fn snare(trigger: Trigger) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.1;
    let noise = noise();
    let detune = 0.0002;
    let noise = (&noise
        + &noise.filter(delay_s(&detune))
        + &noise.filter(delay_s(&detune * 2.0))
        + &noise.filter(delay_s(&detune * 3.0)))
        .filter(compress().ratio(0.1).scale(100.0).build());
    let env = adsr_linear_01(&clock)
        .release_s(duration_s * 1.0)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    let noise = noise.filter(low_pass_moog_ladder(10000.0).resonance(2.0).build());
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 240;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz)
        .reset_trigger(trigger)
        .build();
    (noise + osc).mul_lazy(&env)
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
    let cymbal_loop = cymbal(mk_loop(Key::E, Key::N3, 7));
    sum([kick_loop, snare_loop, cymbal_loop])
        .filter(low_pass_butterworth(input.mouse.x_01() * 10000).build())
}

fn synth_gate_and_midi_index(input: &Input, keys: &[(Key, u8)]) -> (Gate, Su8) {
    struct Entry {
        gate: Gate,
        last_pressed: u64,
        currently_pressed: bool,
        midi_index: u8,
    }
    let state = RefCell::new(
        keys.into_iter()
            .map(|&(key, midi_index)| {
                let gate = input.key(key);
                Entry {
                    gate,
                    last_pressed: 0,
                    currently_pressed: false,
                    midi_index,
                }
            })
            .collect::<Vec<_>>(),
    );
    let midi_index_and_gate_signal = Signal::from_fn(move |ctx| {
        let mut currently_pressed = false;
        let mut midi_index = 0;
        let mut newest_last_pressed = 0;
        let mut midi_index_all = 0;
        let mut newest_last_pressed_all = 0;
        for entry in state.borrow_mut().iter_mut() {
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

fn synth_voice(gate: Gate, midi_index: Su8, filter: Sf64) -> Sf64 {
    let freq_hz = midi_index
        .midi_index_to_freq_hz_a440()
        .filter(low_pass_butterworth(20.0).build());
    let osc = oscillator_hz(Waveform::Saw, &freq_hz).build();
    let env = adsr_linear_01(gate)
        .decay_s(0.5)
        .sustain_01(0.0)
        .release_s(0.1)
        .build();
    let filtered_osc = osc
        .filter(low_pass_butterworth(&freq_hz * 10.0).build())
        .filter(
            low_pass_moog_ladder(&freq_hz * 128.0 * &env * (filter + 0.01))
                .resonance(2.0)
                .build(),
        );
    filtered_osc
}

fn synth_looper(clock: &Trigger, input: &Input) -> Sf64 {
    let (gate, midi_index) = synth_gate_and_midi_index(input, &midi_index_by_key());
    let (gate, midi_index) = clocked_midi_note_monophonic_looper()
        .clock(clock)
        .input_gate(gate)
        .input_midi_index(midi_index)
        .clear(&input.keyboard.slash)
        .length(16)
        .build();
    synth_voice(gate, midi_index, input.mouse.y_01())
}

fn bass_voice(gate: Gate, midi_index: Su8, filter: Sf64) -> Sf64 {
    let freq_hz = midi_index
        .midi_index_to_freq_hz_a440()
        .filter(low_pass_butterworth(10.0).build());
    let mk_lfo = |freq_hz: &Sf64| {
        oscillator_hz(Waveform::Sine, freq_hz / 2)
            .build()
            .signed_to_01()
            * 0.4
            + 0.1
    };
    let mk_pulse = |freq_hz: &Sf64| {
        oscillator_hz(Waveform::Pulse, freq_hz)
            .pulse_width_01(mk_lfo(freq_hz))
            .build()
    };
    let osc = mean([
        mk_pulse(&freq_hz),
        mk_pulse(&(&freq_hz * (1 + 0.37 / &freq_hz))) * 0.5,
        mk_pulse(&(&freq_hz * (1 + 0.47 / &freq_hz))) * 0.5,
    ])
    .filter(low_pass_butterworth(&freq_hz * 8).build());
    let env = adsr_linear_01(gate)
        .attack_s(0.1)
        .release_s(1.0)
        .build()
        .exp_01(-1.0);
    let osc_filtered = osc
        .filter(
            low_pass_moog_ladder(env * freq_hz * 128 * (filter + 0.01))
                .resonance(1.0)
                .build(),
        )
        .map(|x| (x * 8.0).tanh());
    osc_filtered
}

fn bass(input: &Input) -> Sf64 {
    let (gate, midi_index) = synth_gate_and_midi_index(input, &midi_index_by_key_bass());
    bass_voice(gate, midi_index, input.mouse.x_01())
}

fn voice(input: Input) -> Sf64 {
    let clock = periodic_trigger_hz(8.0).build();
    mean([
        drum_looper(&clock, &input),
        synth_looper(&clock, &input),
        bass(&input) * 0.5,
    ])
    .filter(saturate().threshold(1.0).build())
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = voice(window.input());
    window.play(signal)
}
