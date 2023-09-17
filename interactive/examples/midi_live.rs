use clap::{Parser, Subcommand};
use ibis_interactive::{
    envelope::AdsrLinear01,
    filters::*,
    midi::{MidiControllerTable, MidiLive, MidiPlayer, MidiVoice},
    oscillator::{Oscillator, Waveform},
    signal::{self, Sf64},
    window::{Rgb24, Window},
};

fn make_voice(
    MidiVoice {
        note_freq,
        velocity_01,
        gate,
    }: MidiVoice,
    pitch_bend_multiplier: impl Into<Sf64>,
    controllers: &MidiControllerTable,
) -> Sf64 {
    let note_freq_hz = signal::sfreq_to_hz(note_freq) * pitch_bend_multiplier.into();
    let osc = signal::sum([
        Oscillator::builder_hz(Waveform::Saw, &note_freq_hz).build_signal(),
        Oscillator::builder_hz(Waveform::Saw, &note_freq_hz * 1.01).build_signal(),
        Oscillator::builder_hz(Waveform::Saw, &note_freq_hz * 0.99).build_signal(),
    ]);
    let env = AdsrLinear01::builder(&gate)
        .attack_s(0.0)
        .decay_s(2.0)
        .sustain_01(0.5)
        .release_s(0.1)
        .build_signal()
        .exp_01(1.0);
    let filtered_osc = osc.filter(
        LowPassMoogLadder::builder(&env * ((controllers.modulation() * 6000.0) + 4000.0))
            .resonance(9.0)
            .build(),
    );
    filtered_osc.mul_lazy(&env) * velocity_01
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
#[command(name = "midi_live")]
#[command(about = "Example of controlling the synthesizer live with a midi controller")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    ListPorts,
    Play {
        #[arg(short, long)]
        midi_port: usize,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Cli::parse();
    let midi_live = MidiLive::new()?;
    match args.command {
        Commands::ListPorts => {
            for (i, name) in midi_live.enumerate_port_names() {
                println!("{}: {}", i, name);
            }
        }
        Commands::Play { midi_port } => {
            let MidiPlayer {
                voices,
                pitch_bend_multiplier,
                controllers,
                ..
            } = midi_live.into_player(midi_port, 0, 8).unwrap();
            let signal = voices
                .into_iter()
                .map(|voice| make_voice(voice, &pitch_bend_multiplier, &controllers))
                .sum::<Sf64>()
                .filter(Saturate::builder().scale(1.0).threshold(1.0).build());
            run(signal)?;
        }
    }
    Ok(())
}
