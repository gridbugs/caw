use caw_core::*;
use caw_interactive::{Visualization, window::Window};
use caw_midi::{MidiEventsT, MidiMessagesT};
use caw_midi_udp::MidiLiveUdp;
use caw_modules::*;

fn sig(
    pitch_01: Sig<impl SigT<Item = f32>>,
    filter_01: Sig<impl SigT<Item = f32>>,
    resonance_01: Sig<impl SigT<Item = f32>>,
) -> Sig<impl SigT<Item = f32>> {
    super_saw(40.0 + (pitch_01 * 200.))
        .build()
        .filter(low_pass::default(10_000. * filter_01).q(resonance_01))
        .filter(chorus())
        .filter(reverb())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .visualization(Visualization::StereoOscillographics)
        .build();
    let midi_live = MidiLiveUdp::new("0.0.0.0:2000")?;
    let controllers = Sig(midi_live).channel(0.into()).controllers();
    window.play_stereo(
        Stereo::new_fn(|| {
            sig(
                controllers.get_01(0),
                controllers.get_01(1),
                controllers.get_01(2),
            )
        }),
        Default::default(),
    )
}
