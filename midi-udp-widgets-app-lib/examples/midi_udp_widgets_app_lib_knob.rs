use caw_core::*;
use caw_interactive::window::Window;
use caw_midi_udp_widgets_app_lib::knob;
use caw_modules::*;

fn sig() -> Sig<impl SigT<Item = f32>> {
    oscillator(waveform::Saw, 40.0 + 40.0 * knob().title("freq").build())
        .build()
        .zip(oscillator(waveform::Saw, 40.1).build())
        .map(|(a, b)| (a + b) / 10.0)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder().build();
    window.play_mono(sig(), Default::default())
}
