use clap::Parser;
use currawong_interactive::prelude::*;

fn voice(
    VoiceDesc {
        note,
        key_down,
        key_press,
        velocity_01,
    }: VoiceDesc,
) -> Sf64 {
    let note_freq_hz = note.freq_hz();
    let osc = oscillator_hz(Waveform::Pulse, &note_freq_hz).build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.01)
        .decay_s(0.2)
        .sustain_01(0.8)
        .release_s(0.05)
        .build();
    osc.filter(low_pass_moog_ladder(env * 20_000.0).resonance(0.5).build()) * velocity_01
}

fn make_voice(midi_messages: Signal<MidiMessages>) -> Sf64 {
    midi_messages
        .key_events()
        .polyphonic_with(4, 8, voice)
        .mix(|dry| dry.filter(reverb().room_size(0.9).build()))
        .filter(high_pass_butterworth(10.0).build())
}

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

#[derive(Parser, Debug)]
struct Args {
    name: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let midi_file = MidiFile::read(args.name)?;
    run(make_voice(
        midi_file.signal_builder(0, 1.0)?.single_channel(0).build(),
    ))
}
