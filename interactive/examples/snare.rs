use caw_interactive::prelude::*;

fn signal(input: Input) -> Sf64 {
    let basic_snare = |key: Gate| {
        let snare = snare(key.to_trigger_rising_edge()).noise_level(0.5).build();
        snare
            .mix(|dry| 6.0 * dry.filter(low_pass_butterworth(80.0).build()))
            .filter(compress().scale(5.0).ratio(0.1).threshold(1.5).build())
    };
    let gated_reverb = |signal: Sf64| {
        let noise_gate = noise_gate(signal.filter(envelope_follower().build()))
            .threshold(0.1)
            .ratio(0.2)
            .build();
        signal.mix(|dry| {
            let reverb = dry.filter(
                reverb()
                    .room_size(input.mouse.x_01())
                    .damping(input.mouse.y_01())
                    .build(),
            ) * 0.5;
            reverb.filter(noise_gate)
        })
    };
    gated_reverb(basic_snare(input.keyboard.a))
        + basic_snare(input.keyboard.s)
        + kick(input.keyboard.z.to_trigger_rising_edge())
            .noise_level(2.0)
            .build()
        + hat_closed(input.keyboard.x.to_trigger_rising_edge()).build()
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder().sane_default().build();
    window.play(signal(window.input()) * 0.1)
}
