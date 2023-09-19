use clap::Parser;
use ibis_interactive::{
    midi::{MidiFile, MidiPlayer, MidiVoice},
    prelude::*,
    signal::{self, Sf64},
    window::{Rgb24, Window},
};

fn run(signal: Sf64) -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(2.0)
        .stable(true)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    window.play(signal * 0.1)
}

fn make_voice(
    MidiVoice {
        note_freq,
        velocity_01,
        gate,
    }: MidiVoice,
) -> Sf64 {
    let note_freq_hz = signal::sfreq_to_hz(note_freq);
    let osc = vec![
        oscillator_hz(Waveform::Saw, &note_freq_hz).build(),
        oscillator_hz(Waveform::Saw, &note_freq_hz * 1.01).build(),
        oscillator_hz(Waveform::Pulse, &note_freq_hz * 0.5).build(),
    ]
    .into_iter()
    .sum::<Sf64>();
    let env = adsr_linear_01(&gate)
        .attack_s(0.0)
        .decay_s(2.0)
        .sustain_01(0.0)
        .release_s(0.1)
        .build()
        .exp_01(1.0);
    let filtered_osc = osc.filter(low_pass_moog_ladder(&env * 8000.0).resonance(2.0).build());
    filtered_osc.mul_lazy(&env) * velocity_01
}

#[derive(Parser, Debug)]
struct Args {
    name: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let midi_file = MidiFile::read(args.name).unwrap();
    let MidiPlayer { voices, .. } = midi_file.track_player(0, 0, 128, 1.0).unwrap();
    let signal = voices.into_iter().map(make_voice).sum::<Sf64>();
    run(signal)
}
