use caw_core_next::*;
use caw_interactive_next::{Input, Key, Window};
use caw_modules::*;

fn signal(input: Input) -> Sig<impl SigT<Item = f32>> {
    let env = adsr_linear_01(input.keyboard.get(Key::Space))
        .attack_s(0.5)
        .decay_s(0.2)
        .sustain_01(0.5)
        .release_s(2)
        .build();
    (oscillator(waveform::Saw, 100).build()
        + oscillator(waveform::Saw, 101).build())
        * env
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder().sane_default().build();
    let input = window.input();
    window.play_mono(signal(input))
}
