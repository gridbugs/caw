use caw_interactive::prelude::*;

fn voice(input: Input) -> Sf64 {
    let snare_trigger = input.keyboard.q.to_trigger_rising_edge();
    let kick_trigger = input.keyboard.w.to_trigger_rising_edge();
    let hat_closed_trigger = input.keyboard.e.to_trigger_rising_edge();
    snare(snare_trigger).build()
        + kick(kick_trigger).build()
        + hat_closed(hat_closed_trigger).build()
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
