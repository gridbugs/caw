use caw::sample;
use caw_interactive::prelude::*;

#[allow(unused)]
struct Effects {
    x: Sf64,
    y: Sf64,
}

fn filter_chain(signal: Sf64, freq_hz: Sf64, width_ratio: Sf64, effects: &Effects) -> Sf64 {
    let filtered = signal.filter(
        band_pass_chebyshev_centered(&freq_hz)
            .resonance(1.0)
            .width_ratio(width_ratio)
            .build(),
    );
    let env = filtered.filter(envelope_follower().build());
    let osc = oscillator_hz(Waveform::Sine, &freq_hz * (&effects.x + 0.5)).build();
    osc * env
}

fn filter_bank(signal: Sf64, freqs_hz: Vec<f64>, width_ratio: Sf64, effects: &Effects) -> Sf64 {
    freqs_hz
        .into_iter()
        .map(|freq_hz| {
            filter_chain(
                signal.clone(),
                const_(freq_hz),
                width_ratio.clone(),
                effects,
            )
        })
        .sum()
}

fn voice(input: Input) -> Sf64 {
    let path = std::env::args().collect::<Vec<_>>().get(1).unwrap().clone();
    let sample = sample::read_wav(path).unwrap();
    let sampler = sampler(&sample)
        .trigger(input.keyboard.get(Key::Space).to_trigger_rising_edge())
        .build();
    let a = 6.0;
    let freqs_hz = (0..36).map(|i| 2f64.powf(i as f64 / a + 7.0)).collect();
    let effects = Effects {
        x: input.mouse.x_01(),
        y: input.mouse.y_01(),
    };
    filter_bank(sampler, freqs_hz, const_(0.25 / a), &effects) * 4.0
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder().sane_default().width_px(500).build();
    window.play(voice(window.input()))
}
