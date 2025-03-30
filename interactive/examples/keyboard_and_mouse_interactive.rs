use caw_core::*;
use caw_interactive::{Input, MouseButton, Visualization, Window};
use caw_keyboard::{IntoNoteFreqHz, KeyEventsT, MonoVoice, Note};
use caw_modules::*;

fn sig(input: Input, channel: Channel) -> Sig<impl SigT<Item = f32> + Send> {
    let MonoVoice {
        note,
        key_down_gate,
        key_press_trig,
        ..
    } = input
        .keyboard
        .opinionated_key_events(Note::B0)
        .debug(|x| {
            if !x.is_empty() {
                //println!("{:?}", x);
            }
        })
        .mono_voice();
    let key_press_trig = key_press_trig.shared();
    let key_down_gate = key_down_gate | input.mouse.button(MouseButton::Left);
    let env = adsr_linear_01(key_down_gate)
        .key_press_trig(key_press_trig.clone())
        .attack_s(0.01)
        .decay_s(0.2)
        .sustain_01(0.8)
        .release_s(10.0)
        .build()
        .exp_01(1.);
    let osc =
        super_saw(note.freq_hz().filter(low_pass::butterworth(5.))).build();
    osc.filter(
        low_pass::default(env * input.mouse.y_01() * 10000.)
            .resonance(input.mouse.x_01()),
    )
    .filter(
        chorus()
            .lfo_rate_hz(0.1)
            .num_voices(1)
            .delay_s(0.001)
            .lfo_offset(ChorusLfoOffset::Interleave(channel))
            .lfo_reset_trig(key_press_trig.clone()),
    )
    .filter(reverb::default().room_size(0.9).damping(0.9))
    .filter(high_pass::default(1.))
    .filter(compressor().threshold(0.1).scale(2.0).ratio(0.1))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .scale(4.)
        .build();
    let input = window.input();
    window.play_stereo(
        Stereo::new_fn_channel(|channel| sig(input.clone(), channel)),
        Default::default(),
    )
}
