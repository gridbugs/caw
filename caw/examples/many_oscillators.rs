use caw::prelude::*;

fn run(signal: Sf64) -> anyhow::Result<()> {
    let mut signal_player = SignalPlayer::new()?;
    signal_player.play_sample_forever(signal);
}

fn signal() -> Sf64 {
    let n = 300;
    (0..n)
        .map(move |i| {
            oscillator_hz(Waveform::Saw, 100.0 + ((i as f64 / n as f64) * 1.0))
                .build()
                .map(move |x| (x / (n as f64)) * 10.0)
        })
        .sum()
}

fn main() -> anyhow::Result<()> {
    run(signal())
}
