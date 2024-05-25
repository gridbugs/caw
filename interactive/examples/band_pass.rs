use caw_interactive::prelude::*;

fn voice(input: Input) -> Sf64 {
    let modulation = input
        .mouse
        .x_01()
        .max(0.1)
        .filter(low_pass_butterworth(2.0).build());
    let osc = supersaw_hz(200.0).build();
    (osc.filter(
        band_pass_chebyshev_centered(1000.0 * &modulation)
            .width_ratio(0.2)
            .resonance(2.0)
            .build(),
    ) + osc.filter(
        band_pass_chebyshev_centered(1500.0 * &modulation)
            .width_ratio(0.2)
            .resonance(2.0)
            .build(),
    ) * 5.0)
        .filter(saturate().threshold(2.0).build())
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(2.0)
        .stable(true)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    window.play(voice(window.input()) * 0.1)
}
