use clap::Parser;
use ibis_interactive::{
    envelope::AdsrLinear01,
    filters::*,
    midi::{MidiFile, MidiPlayer, MidiVoice},
    oscillator::{Oscillator, Waveform},
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
        Oscillator::builder_hz(Waveform::Saw, &note_freq_hz).build_signal(),
        Oscillator::builder_hz(Waveform::Saw, &note_freq_hz * 1.01).build_signal(),
        Oscillator::builder_hz(Waveform::Saw, &note_freq_hz * 0.5).build_signal(),
        Oscillator::builder_hz(Waveform::Saw, &note_freq_hz * 0.505).build_signal(),
    ]
    .into_iter()
    .sum::<Sf64>();
    let env = AdsrLinear01::builder(&gate)
        .attack_s(0.0)
        .decay_s(1.0)
        .sustain_01(0.5)
        .release_s(0.1)
        .build_signal();
    let filtered_osc = osc.filter(
        LowPassMoogLadder::builder((&env * 8000.0) + 0.0)
            .resonance(2.0)
            .build(),
    );
    filtered_osc.mul_lazy(&env) * velocity_01
}

#[derive(Parser, Debug)]
struct Args {
    name: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let midi_file = MidiFile::read(args.name).unwrap();
    let MidiPlayer { voices, .. } = midi_file.track_player(0, 0, 128).unwrap();
    let signal = voices.into_iter().map(make_voice).sum::<Sf64>();
    run(signal)
}
