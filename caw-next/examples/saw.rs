use caw_builders::*;
use caw_core_next::*;
use caw_next::player::Player;

fn signal() -> SigBuf<impl Sig<Item = f32, Buf = Vec<f32>>> {
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
