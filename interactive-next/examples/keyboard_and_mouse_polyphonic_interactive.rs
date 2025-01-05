use caw_core_next::*;
use caw_interactive_next::{Input, Visualization, Window};
use caw_keyboard::{IntoNoteFreqHz, KeyEventsT, MonoVoice, Note};
use caw_modules::*;

fn chorus(
    input: Input,
    channel: Channel,
    sig: impl SigT<Item = f32>,
) -> Sig<impl SigT<Item = f32>> {
    let sig = sig_shared(sig);
    let lfo = oscillator(Sine, 0.5)
        .reset_offset_01(channel.circle_phase_offset_01())
        .build();
    let delay_s = 0.001 + ((lfo + 1.0) * 0.001);
    sig.clone().filter(
        delay_periodic_s(delay_s)
            .feedback_ratio(0.5)
            .mix_01(input.mouse.x_01()),
    )
}

fn sig(input: Input, ch: Channel) -> Sig<impl SigT<Item = f32>> {
    chorus(
        input.clone(),
        ch,
        input
            .clone()
            .keyboard
            .opinionated_key_events(Note::B2)
            .poly_voices(12)
            .into_iter()
            .map(
                move |MonoVoice {
                          note,
                          key_down_gate,
                          key_press_trigger,
                          ..
                      }| {
                    let key_down_gate = key_down_gate.shared();
                    let key_press_trigger = key_press_trigger.shared();
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
                        low_pass::default(filter_env * 15_000.).resonance(0.),
                    ) * amp_env
                },
            )
            .sum::<Sig<_>>(),
    )
    .filter(reverb::default().room_size(0.9).damping(0.5))
    .filter(high_pass::default(1.))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .scale(0.5)
        .stride(2)
        .build();
    let input = window.input();
    window.play_stereo(
        Stereo::new_fn_channel(|ch| sig(input.clone(), ch)),
        Default::default(),
    )
}
