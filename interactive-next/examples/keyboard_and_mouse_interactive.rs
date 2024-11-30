use caw_core_next::*;
use caw_interactive_next::{Input, MouseButton, Visualization, Window};
use caw_keyboard::{IntoNoteFreqHz, KeyEventsT, MonoVoice, Note};
use caw_modules::*;

fn sig(input: Input) -> Sig<impl SigT<Item = f32>> {
    let MonoVoice {
        note,
        key_down_gate,
        key_press_trigger,
        ..
    } = input.keyboard.opinionated_key_events(Note::B0).mono_voice();
    let key_down_gate = key_down_gate | input.mouse.button(MouseButton::Left);
    let env = adsr_linear_01(key_down_gate)
        .key_press_trig(key_press_trigger)
        .attack_s(0.01)
        .decay_s(0.2)
        .sustain_01(0.8)
        .release_s(10.0)
        .build()
        .exp_01(1);
    let osc = super_saw(note.freq_hz()).build();
    osc.filter(
        low_pass::default(env * input.mouse.y_01() * 10000)
            .resonance(input.mouse.x_01()),
    )
    .filter(reverb::default().room_size(0.9).damping(0.9))
    .filter(high_pass::default(1))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .build();
    let input = window.input();
    window.play_stereo(
        sig(input.clone()),
        sig(input.clone()),
        Default::default(),
    )
}
