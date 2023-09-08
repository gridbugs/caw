use ibis_interactive::{
    envelope::AdsrLinear01,
    filters::*,
    input::{Input, Key},
    music::{self, Note, NoteName},
    oscillator::{Oscillator, Waveform},
    signal::{Gate, Sf64},
    window::{Rgb24, Window},
};

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
        .map(|(i, key)| (key, music::freq_hz_of_midi_index(i as u8 + top_row_base)))
        .chain(
            bottom_row
                .into_iter()
                .enumerate()
                .map(|(i, key)| (key, music::freq_hz_of_midi_index(i as u8 + bottom_row_base))),
        )
        .collect::<Vec<_>>()
}

fn single_voice(freq_hz: f64, gate: Gate, effect_x: Sf64, effect_y: Sf64) -> Sf64 {
    let oscillator = Oscillator::builder_hz(Waveform::Saw, freq_hz).build_signal()
        + Oscillator::builder_hz(Waveform::Saw, freq_hz * 1.01).build_signal()
        + Oscillator::builder_hz(Waveform::Pulse, freq_hz * 2.0).build_signal() * 0.3
        + Oscillator::builder_hz(Waveform::Pulse, freq_hz * 2.004).build_signal() * 0.3
        + Oscillator::builder_hz(Waveform::Pulse, freq_hz * 2.003).build_signal() * 0.3
        + Oscillator::builder_hz(Waveform::Saw, freq_hz)
            .reset_offset_01(0.5)
            .build_signal();
    let amp_env = AdsrLinear01::builder(gate.clone())
        .release_s(0.5)
        .build_signal();
    let filter_env = AdsrLinear01::builder(gate.clone())
        .attack_s(0.05)
        .decay_s(0.1)
        .sustain_01(0.6)
        .release_s(0.5)
        .build_signal()
        .exp_01(1.0);
    oscillator
        .filter(
            LowPassMoogLadder::builder(&filter_env * 12000.0 * effect_x)
                .resonance(effect_y * 4.0)
                .build(),
        )
        .mul_lazy(&amp_env)
        .force(&filter_env)
}

fn voice(input: Input) -> Sf64 {
    freq_hz_by_gate()
        .into_iter()
        .map(|(key, freq_hz)| {
            single_voice(
                freq_hz,
                input.key(key),
                input.x_01().clone(),
                input.y_01().clone(),
            )
        })
        .sum::<Sf64>()
        .filter(Saturate::builder().scale(2.0).min(-1.0).max(2.0).build())
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
