use caw_interactive::prelude::*;

fn voice(input: Input) -> Sf64 {
    let leader_osc =
        oscillator_hz(Waveform::Pulse, input.mouse.x_01() * 1000.0)
            .pulse_width_01(0.1)
            .build();
    
    oscillator_hz(Waveform::Saw, input.mouse.y_01() * 1000.0)
            .hard_sync(&leader_osc)
            .build()
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
