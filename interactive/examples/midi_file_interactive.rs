use caw_core::*;
use caw_interactive::{Input, MouseButton, Visualization, Window};
use caw_keyboard::{IntoNoteFreqHz, KeyEventsT, MonoVoice};
use caw_midi_file::*;
use caw_modules::*;
use clap::Parser;

fn sig(
    input: Input,
    midi_messages: FrameSig<impl FrameSigT<Item = MidiMessages>>,
    channel: Channel,
) -> Sig<impl SigT<Item = f32>> {
    midi_messages
        .key_events()
        .poly_voices(48)
        .into_iter()
        .map(
            |MonoVoice {
                 note,
                 key_down_gate,
                 key_press_trig,
                 ..
             }| {
                let key_down_gate =
                    key_down_gate | input.mouse.button(MouseButton::Left);
                let env = adsr_linear_01(key_down_gate)
                    .key_press_trig(key_press_trig)
                    .attack_s(0.01)
                    .decay_s(0.2)
                    .sustain_01(0.9)
                    .release_s(0.1)
                    .build();
                let osc = super_saw(note.freq_hz()).num_oscillators(32).build();
                let _offset = match channel {
                    Channel::Left => 0.0,
                    Channel::Right => 0.25,
                };
                osc.filter(
                    low_pass::default(env * 20000.)
                        .resonance(input.mouse.x_01()),
                )
            },
        )
        .sum::<Sig<_>>()
        .filter(reverb::default().room_size(0.9).damping(0.9))
        .filter(high_pass::default(1.))
        * 0.25
}

#[derive(Parser, Debug)]
struct Args {
    name: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();
    let midi_file = MidiFile::read(args.name)?;
    let midi_messages = midi_file.track(0, 1.0)?.into_midi_messages(0).shared();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .fade(true)
        .build();
    let input = window.input();
    window.play_stereo(
        Stereo::new_fn_channel(|channel| {
            sig(input.clone(), midi_messages.clone(), channel)
        }),
        Default::default(),
    )
}
