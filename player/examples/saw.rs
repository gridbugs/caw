use caw_core::*;
use caw_modules::*;
use caw_player::Player;

fn signal() -> Sig<impl SigT<Item = f32>> {
    oscillator(waveform::Saw, 40.0)
        .build()
        .zip(oscillator(waveform::Saw, 40.1).build())
        .map(|(a, b)| (a + b) / 10.0)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let player = Player::new()?;
    let _handle = player.play_mono(signal(), Default::default())?;
    std::thread::park();
    Ok(())
}
