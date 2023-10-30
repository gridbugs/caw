use currawong_interactive::prelude::*;

fn freq_hz_by_gate() -> Vec<(Key, f64)> {
    use Key::*;
    let top_row_base = Note::new(NoteName::A, 2).to_midi_index();
    let top_row = vec![
        N1,
        Q,
        N2,
        W,
        E,
        N4,
        R,
        N5,
        T,
        Y,
        N7,
        U,
        N8,
        I,
        N9,
        O,
        P,
        Minus,
        LeftBracket,
        Equals,
        RightBracket,
    ];
    let bottom_row_base = Note::new(NoteName::B, 1).to_midi_index();
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

fn single_voice(freq_hz: Sf64, gate: Gate, effect_x: Sf64, effect_y: Sf64) -> Sf64 {
    let lfo = oscillator_hz(Waveform::Sine, 128.0 * effect_y)
        .build()
        .signed_to_01();
    let oscillator = oscillator_hz(Waveform::Saw, &freq_hz).build();
    let env = adsr_linear_01(&gate)
        .decay_s(0.5)
        .sustain_01(0.5)
        .release_s(10.0)
        .build()
        .exp_01(2.0);
    let lfo_env = adsr_linear_01(&gate)
        .attack_s(1.0)
        .release_s(5.0)
        .build()
        .exp_01(4.0);
    oscillator
        .filter(
            low_pass_moog_ladder(
                (env * (20000.0 - (lfo * lfo_env * 20000.0 * effect_x))).clamp_non_negative(),
            )
            .resonance(1.3)
            .build(),
        )
        .map(|x| (x * 8.0).tanh())
}

fn voice(input: Input) -> Sf64 {
    struct Entry {
        gate: Gate,
        last_pressed: u64,
        currently_pressed: bool,
        freq_hz: f64,
    }
    let mut state = freq_hz_by_gate()
        .into_iter()
        .map(|(key, freq_hz)| {
            let gate = input.key(key);
            Entry {
                gate,
                last_pressed: 0,
                currently_pressed: false,
                freq_hz,
            }
        })
        .collect::<Vec<_>>();
    let freq_hz_and_gate = Signal::from_fn(move |ctx| {
        let mut currently_pressed = false;
        let mut freq_hz = 0.0;
        let mut newest_last_pressed = 0;
        let mut freq_hz_all = 0.0;
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
                    freq_hz = entry.freq_hz;
                }
            } else {
                entry.currently_pressed = false;
            }
            if entry.last_pressed > newest_last_pressed_all {
                newest_last_pressed_all = entry.last_pressed;
                freq_hz_all = entry.freq_hz;
            }
        }
        let freq_hz = if currently_pressed {
            freq_hz
        } else {
            freq_hz_all
        };
        (freq_hz, currently_pressed)
    });
    let freq_hz = freq_hz_and_gate.map(|x| x.0);
    let gate = freq_hz_and_gate.map(|x| x.1).to_gate();
    single_voice(freq_hz, gate, input.x_01().clone(), input.y_01().clone())
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
