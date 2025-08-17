use caw_core::*;
use caw_interactive::{Input, Visualization, Window};
use caw_keyboard::{IntoNoteFreqHz, KeyEventsT, MonoVoice, Note};
use caw_modules::*;
use caw_player::PlayerConfig;

fn sig(input: Input, ch: Channel) -> Sig<impl SigT<Item = f32> + Send> {
    input
        .clone()
        .keyboard
        .opinionated_key_events(Note::B1)
        .poly_voices(2)
        .into_iter()
        .map(
            move |MonoVoice {
                      note,
                      key_down_gate,
                      key_press_trig,
                      ..
                  }| {
                let key_down_gate = key_down_gate.shared();
                let key_press_trigger = key_press_trig.shared();
                let filter_env = adsr_linear_01(key_down_gate.clone())
                    .key_press_trig(key_press_trigger.clone())
                    .attack_s(0.2)
                    .release_s(4.)
                    .build();
                let amp_env = adsr_linear_01(key_down_gate.clone())
                    .key_press_trig(key_press_trigger.clone())
                    .attack_s(0.075)
                    .release_s(1.)
                    .build()
                    .exp_01(1.);
                let osc = super_saw(note.freq_hz())
                    .num_oscillators(2)
                    .init(SuperSawInit::Const(ch.circle_phase_offset_01()))
                    .detune_ratio(0.008)
                    .build();
                osc.filter(
                    low_pass::default(filter_env * 20_000.).resonance(0.),
                ) * amp_env
            },
        )
        .sum::<Sig<_>>()
        .filter(
            chorus()
                .num_voices(3)
                .lfo_rate_hz(0.5)
                .delay_s(input.mouse.x_01() * 0.01)
                .depth_s(input.mouse.y_01() * 0.01)
                .lfo_offset(ChorusLfoOffset::Interleave(ch))
                .mix_01(0.5)
                .feedback_ratio(0.5),
        )
        .filter(reverb::default().room_size(0.5).damping(0.5))
        .filter(high_pass::default(1.))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(3)
        .scale(0.7)
        .stride(2)
        .build();
    let input = window.input();
    window.play_stereo(
        Stereo::new_fn_channel(|ch| sig(input.clone(), ch)),
        PlayerConfig {
            system_latency_s: 0.05,
            ..Default::default()
        },
    )
}
