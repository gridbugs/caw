use clap::{Parser, Subcommand};
use currawong_interactive::prelude::*;

#[derive(Clone)]
struct Effects {
    low_pass_cutoff: Sf64,
    low_pass_resonance: Sf64,
}

impl Effects {
    fn new(_controllers: MidiControllerTable, input: Input) -> Self {
        Self {
            low_pass_cutoff: input.mouse.x_01(),
            low_pass_resonance: input.mouse.y_01(),
        }
    }
}

fn voice(
    VoiceDesc {
        note,
        key_down,
        key_press,
        ..
    }: VoiceDesc,
    pitch_bend_multiplier_hz: Sf64,
    effects: Effects,
) -> Sf64 {
    let osc = oscillator_hz(Waveform::Saw, note.freq_hz() * pitch_bend_multiplier_hz).build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.0)
        .decay_s(0.2)
        .sustain_01(0.5)
        .release_s(4.0)
        .build()
        .exp_01(1.0);
    osc.filter(
        low_pass_moog_ladder(env * (30 * note.freq_hz() * effects.low_pass_cutoff))
            .resonance(4.0 * effects.low_pass_resonance)
            .build(),
    )
}

fn make_voice(midi_messages: Signal<MidiMessages>, input: Input) -> Sf64 {
    let pitch_bend_multiplier_hz = midi_messages.pitch_bend_multiplier_hz();
    let effects = Effects::new(midi_messages.controller_table(), input);
    midi_messages
        .key_events()
        .voice_descs_polyphonic(3, 5)
        .into_iter()
        .map(|voice_desc| {
            voice(
                voice_desc,
                pitch_bend_multiplier_hz.clone(),
                effects.clone(),
            )
        })
        .sum::<Sf64>()
        .mix(|dry| dry.filter(reverb().room_size(1.0).damping(0.5).build()))
        .filter(high_pass_butterworth(10.0).build())
}

fn run(midi_messages: Signal<MidiMessages>) -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(1.0)
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = make_voice(midi_messages, window.input());
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
            let midi_messages = midi_live
                .signal_builder(midi_port)?
                .single_channel(0)
                .build();
            run(midi_messages)?
        }
    }
    Ok(())
}
