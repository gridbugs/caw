use caw_core::*;
use caw_interactive::{Input, Visualization, window::Window};
use caw_keyboard::{IntoNoteFreqHz, KeyEventsT, MonoVoice, Note};
use caw_modules::*;
use caw_utils::*;

fn sig(input: Input) -> StereoPair<Sig<impl SigT<Item = f32>>> {
    let y = cell(input.mouse().y_01());
    let x = cell(input.mouse().x_01());
    let MonoVoice { note, .. } = input
        .keyboard
        .opinionated_key_events(Note::B_0)
        .mono_voice();
    let note = cell(note);
    let tempo_hz = cell_f32(8.);
    let clock = cell(periodic_trig_hz(tempo_hz.clone()).build());
    let seq = cell(value_sequencer(clock.clone(), [1., 2., 3., 1.5]));
    let amp_seq = cell(value_sequencer(
        clock.clone(),
        [1., 0.2, 0.5, 0.9, 0.75, 0.5, 0.2, 0.75],
    ));
    Stereo::new_fn(|| {
        let freq_mul = seq.clone();
        let amp = amp_seq.clone();
        let freq_hz = note.clone().freq_hz() * freq_mul;
        let osc = saw(freq_hz).build();
        let env = adsr_linear_01(clock.clone()).r(0.15).build().exp_01(1.0);
        osc.filter(high_pass::default(100.))
            .filter(
                low_pass::diode_ladder(20_000. * env * amp * x.clone())
                    .q(y.clone()),
            )
            .map(|x| (x * 5.).tanh() / 5.)
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
