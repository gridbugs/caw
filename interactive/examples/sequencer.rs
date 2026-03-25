use caw_core::*;
use caw_interactive::{Input, Visualization, window::Window};
use caw_keyboard::{IntoNoteFreqHz, KeyEventsT, MonoVoice, Note, note};
use caw_modules::*;
use caw_utils::*;

#[derive(Default)]
struct Step {
    note: Note,
    octave: i8,
    accent: bool,
    slide: bool,
}

fn sequence() -> Vec<Step> {
    vec![
        Step {
            note: note::D_4,
            octave: -1,
            ..Default::default()
        },
        Step {
            note: note::F_4,
            octave: 1,
            ..Default::default()
        },
        Step {
            note: note::D_4,
            slide: true,
            ..Default::default()
        },
        Step {
            note: note::A_SHARP_3,
            octave: 1,
            ..Default::default()
        },
        Step {
            note: note::A_SHARP_3,
            accent: true,
            slide: true,
            ..Default::default()
        },
        Step {
            note: note::F_4,
            octave: -1,
            ..Default::default()
        },
        Step {
            note: note::C_4,
            octave: -1,
            accent: true,
            slide: true,
            ..Default::default()
        },
        Step {
            note: note::A_SHARP_3,
            octave: -1,
            ..Default::default()
        },
        Step {
            note: note::A_SHARP_3,
            octave: 1,
            slide: true,
            ..Default::default()
        },
        Step {
            note: note::D_4,
            ..Default::default()
        },
        Step {
            note: note::C_4,
            octave: 1,
            slide: true,
            ..Default::default()
        },
        Step {
            note: note::D_4,
            octave: 1,
            accent: true,
            slide: true,
            ..Default::default()
        },
        Step {
            note: note::D_4,
            ..Default::default()
        },
        Step {
            note: note::D_4,
            slide: true,
            ..Default::default()
        },
        Step {
            note: note::D_4,
            octave: -1,
            slide: true,
            ..Default::default()
        },
        Step {
            note: note::D_SHARP_4,
            slide: true,
            ..Default::default()
        },
    ]
}

fn sig(input: Input) -> StereoPair<Sig<impl SigT<Item = f32>>> {
    let y = cell(input.mouse().y_01());
    let x = cell(input.mouse().x_01());
    let MonoVoice { note, .. } = input
        .keyboard
        .opinionated_key_events(Note::B_0)
        .mono_voice();
    let bpm = Sig(111.0 * 4.0);
    let clock = cell(periodic_trig_bpm(bpm.clone()).build());
    let seq = sequence();
    let freqs = seq
        .iter()
        .map(|step| {
            let note = step.note.add_octaves(step.octave - 1);
            note.freq_hz()
        })
        .collect::<Vec<_>>();
    let freqs_rotated = {
        let mut v = freqs[1..].to_vec();
        v.push(freqs[0].clone());
        v
    };
    let slide_mults = seq
        .iter()
        .map(|step| if step.slide { 1.0 } else { 0.0 })
        .collect::<Vec<_>>();
    let accent_mults = seq
        .iter()
        .map(|step| if step.accent { 1.0f32 } else { 0.0 })
        .collect::<Vec<_>>();
    let accents_rotated = {
        let mut v = accent_mults[1..].to_vec();
        v.push(accent_mults[0].clone());
        v
    };
    let accent_seq = cell(value_sequencer(clock.clone(), accent_mults));
    let next_accent_seq = cell(value_sequencer(clock.clone(), accents_rotated));
    let slide_seq = cell(value_sequencer(clock.clone(), slide_mults));
    let note_seq = cell(value_sequencer(clock.clone(), freqs));
    let next_note_seq = cell(value_sequencer(clock.clone(), freqs_rotated));
    Stereo::new_fn(|| {
        let this_freq_hz = note_seq.clone();
        let next_freq_hz = next_note_seq.clone();
        let slide_mul = slide_seq.clone();
        let this_accent_mul = accent_seq.clone();
        let next_accent_mul = next_accent_seq.clone();
        let period_s = bpm.clone().bpm_to_period_s().shared();
        let slide_ratio = 0.8;
        let pause_s = (period_s.clone() * 0.5)
            - (slide_mul.clone() * period_s.clone() * (slide_ratio / 2.0));
        let attack_s = slide_mul.clone() * period_s.clone() * slide_ratio;
        let slide_env = adsr_linear_01(true)
            .clear_trig(clock.clone())
            .pause_s(pause_s)
            .attack_s(attack_s)
            .build()
            .signed_from_01()
            .tanh_mul_clamped(2.)
            .signed_to_01()
            .shared();
        let freq_hz = this_freq_hz.clone()
            + ((next_freq_hz - this_freq_hz) * slide_env.clone());
        let accent_env = adsr_linear_01(true)
            .clear_trig(clock.clone())
            .pause_s(period_s.clone() * 0.5)
            .build()
            .shared();
        let accent_mul = (this_accent_mul.clone()
            + ((next_accent_mul - this_accent_mul) * accent_env.clone()))
        .shared();
        let env_trigger = slide_env
            .clone()
            .map(|x| x > 0.5)
            .gate_to_trig_rising_edge()
            .shared();
        let osc = saw(freq_hz).build();
        let base_filter_env = adsr_linear_01(
            env_trigger.clone().trig_to_gate(period_s.clone() * 0.6),
        )
        .attack_s(period_s.clone() * 0.0)
        .release_s(period_s.clone() * 2.0)
        .build();

        let accent_trigger = accent_mul
            .clone()
            .map(|x| x > 0.5)
            .gate_to_trig_rising_edge()
            .shared();

        let accent_filter_env = adsr_linear_01(
            accent_trigger.clone().trig_to_gate(period_s.clone() * 0.2),
        )
        .attack_s(period_s.clone() * 0.2)
        .release_s(period_s.clone() * 2.0)
        .build();

        let accent_amp_env = adsr_linear_01(
            accent_trigger.clone().trig_to_gate(period_s.clone() * 0.2),
        )
        .attack_s(period_s.clone() * 0.2)
        .release_s(period_s.clone() * 2.0)
        .build();

        let base = osc
            .filter(
                low_pass::diode_ladder(
                    (200.
                        + base_filter_env * 4_000.
                        + accent_filter_env * 4_000.)
                        * x.clone(),
                )
                .q(2.0 * y.clone()),
            )
            .filter(high_pass::default(100.0));
        (base * (0.7 + (accent_amp_env * 0.3)))
            .filter(chorus())
            .mul_tanh(10.0)
            .filter(reverb())
    })
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(2.0)
        .visualization(Visualization::Oscilloscope)
        .build();
    window.play_stereo(sig(window.input()), Default::default())
}
