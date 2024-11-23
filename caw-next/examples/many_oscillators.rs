use caw_core_next::*;
use caw_modules::*;
use caw_next::player::Player;
use rand::Rng;

fn osc(freq: f32) -> Sig<impl SigT<Item = f32>> {
    oscillator(waveform::Saw, freq)
        .reset_offset_01(rand::thread_rng().gen::<f32>() * 0.1)
        .build()
}

fn signal() -> Sig<impl SigT<Item = f32>> {
    let n = 6000;
    (0..n)
        .map(move |i| {
            osc(60.0 + ((i as f32 / n as f32) * 0.1))
                .map(move |x| (x / (n as f32)) * 10.0)
        })
        .sum()
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let player = Player::new()?;
    player.play_signal_sync_mono(signal(), Default::default())
}
