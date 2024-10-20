use caw_builders::*;
use caw_core_next::*;
use caw_next::player::Player;

fn signal(
) -> BufferedSignal<impl SignalTrait<Item = f32, SampleBuffer = Vec<f32>>> {
    oscillator(waveform::Saw, freq_hz(40.0))
        .build()
        .zip(oscillator(waveform::Saw, freq_hz(40.1)).build())
        .map(|(a, b)| (a + b) / 10.0)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let player = Player::new()?;
    player.play_signal_sync(signal())
}
