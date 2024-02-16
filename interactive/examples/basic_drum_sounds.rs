use currawong_interactive::prelude::*;

fn voice(input: Input) -> Sf64 {
    let snare_trigger = input.keyboard.q.to_trigger_rising_edge();
    let kick_trigger = input.keyboard.w.to_trigger_rising_edge();
    let closed_hat_trigger = input.keyboard.e.to_trigger_rising_edge();
    currawong_core::patches::snare_drum(snare_trigger)
        + currawong_core::patches::kick_drum(kick_trigger)
        + currawong_core::patches::closed_hat(closed_hat_trigger)
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
