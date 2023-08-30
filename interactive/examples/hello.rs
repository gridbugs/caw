use ibis_interactive::{
    modules::{Oscillator, Waveform},
    signal::Trigger,
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
    let signal = Oscillator {
        waveform: Waveform::Saw.into(),
        frequency_hz: 440.0.into(),
        pulse_width_01: 0.0.into(),
        reset_trigger: Trigger::never(),
        reset_offset_01: 0.0.into(),
    }
    .signal();
    window.run(|| {
        player.send_signal(&mut signal.map(|x| x as f32));
    })
}
