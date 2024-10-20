use caw_builders::*;
use caw_core_next::*;
use caw_next::player::Player;

fn osc(
    freq: f64,
) -> BufferedSignal<impl SignalTrait<Item = f64, SampleBuffer = Vec<f64>>> {
    oscillator(waveform::Saw, freq_hz(freq)).build().buffered()
}

fn signal(
) -> BufferedSignal<impl SignalTrait<Item = f64, SampleBuffer = Vec<f64>>> {
    let n = 5000;
    (0..n)
        .map(move |i| {
            osc(100.0 + ((i as f64 / n as f64) * 1.0))
                .map(move |x| (x / (n as f64)) * 10.0)
        })
        .sum()
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let player = Player::new()?;
    player.play_signal_sync(signal().signal)
}
