use caw_interactive::prelude::*;

fn freq_hz_by_gate() -> Vec<(Key, f64)> {
    use Key::*;
    let top_row_base = Note::new(NoteName::A, OCTAVE_2).to_midi_index();
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
    let bottom_row_base = Note::new(NoteName::B, OCTAVE_1).to_midi_index();
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

fn single_voice(freq_hz: f64, gate: Gate, effect_x: Sf64, effect_y: Sf64) -> Sf64 {
    let freq_hz = const_(freq_hz);
    let oscillator = oscillator_hz(Waveform::Saw, &freq_hz).build()
        + oscillator_hz(Waveform::Saw, &freq_hz)
            .reset_offset_01(0.5)
            .build();
    let amp_env = adsr_linear_01(&gate).release_s(0.5).build();
    let filter_env = adsr_linear_01(&gate)
        .attack_s(0.0)
        .decay_s(0.1)
        .sustain_01(0.2)
        .release_s(0.5)
        .build();
    oscillator
        .filter(
            low_pass_moog_ladder(&filter_env * 5000.0)
                .resonance(4.0)
                .build(),
        )
        .filter(quantize(1.0 + 10.0 * effect_y).build())
        .filter(down_sample(1.0 + (100.0 * effect_x)).build())
        .filter(low_pass_moog_ladder(10000.0).build())
        .mul_lazy(&amp_env)
        .force_lazy(&filter_env)
}

fn voice(input: Input) -> Sf64 {
    let effect_x = input.mouse.x_01();
    let effect_y = input.mouse.y_01();
    freq_hz_by_gate()
        .into_iter()
        .map(|(key, freq_hz)| {
            single_voice(freq_hz, input.key(key), effect_x.clone(), effect_y.clone())
        })
        .sum::<Sf64>()
        .filter(compress().ratio(0.1).scale(2.0).build())
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
