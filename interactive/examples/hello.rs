use ibis_interactive::{
    modules::{Oscillator, Waveform},
    signal_player::SignalPlayer,
    window::Window,
};

fn main() -> anyhow::Result<()> {
    let window = Window {
        title: "hello".to_string(),
        width_px: 960,
        height_px: 720,
    };
    let mut player = SignalPlayer::new()?;
    let mut signal = Oscillator::builder(Waveform::Pulse, 120.0)
        .pulse_width_01(0.2)
        .signal();
    window.run(|| {
        player.send_signal(&mut signal);
    })
}
