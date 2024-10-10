use caw::prelude::*;

fn run(signal: Sf64) -> anyhow::Result<()> {
    let mut signal_player = SignalPlayer::new()?;
    signal_player.play_sample_forever(signal);
}

fn main() -> anyhow::Result<()> {
    run(oscillator_hz(Waveform::Saw, 100.0).build())
}
