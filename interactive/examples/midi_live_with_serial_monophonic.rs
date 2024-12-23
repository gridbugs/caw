use caw_interactive::prelude::*;
use clap::{Parser, Subcommand};

struct Effects {
    low_pass_cutoff: Sf64,
    low_pass_resonance: Sf64,
}

impl Effects {
    fn new(
        _controllers: MidiControllerTable,
        _extra_controllers: MidiControllerTable,
        input: Input,
    ) -> Self {
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
    let osc = supersaw_hz(note.freq_hz() * pitch_bend_multiplier_hz).build();
    let env = adsr_linear_01(&key_down)
        .key_press(key_press)
        .attack_s(0.0)
        .decay_s(0.2)
        .sustain_01(0.5)
        .release_s(4.0)
        .build()
        .exp_01(1.0);
    osc.filter(
        low_pass_moog_ladder(
            env * (30 * note.freq_hz() * effects.low_pass_cutoff),
        )
        .resonance(4.0 * effects.low_pass_resonance)
        .build(),
    )
    .mix(|dry| dry.filter(reverb().room_size(1.0).damping(0.5).build()))
        * 2.0
}

fn make_voice(
    midi_messages: Signal<MidiMessages>,
    serial_midi_messages: Signal<MidiMessages>,
    input: Input,
) -> Sf64 {
    let keyboard_voice_desc =
        midi_messages.key_events().voice_desc_monophonic();
    let pitch_bend_multiplier_hz = midi_messages.pitch_bend_multiplier_hz();
    let effects = Effects::new(
        midi_messages.controller_table(),
        serial_midi_messages.controller_table(),
        input,
    );
    voice(keyboard_voice_desc, pitch_bend_multiplier_hz, effects)
}

fn run(
    midi_messages: Signal<MidiMessages>,
    serial_midi_messages: Signal<MidiMessages>,
) -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(1.0)
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal =
        make_voice(midi_messages, serial_midi_messages, window.input());
    window.play(signal * 0.1)
}

#[derive(Parser, Debug)]
#[command(name = "midi_live")]
#[command(
    about = "Example of controlling the synthesizer live with a midi controller"
)]
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
        #[arg(short, long)]
        serial_port: String,
        #[arg(long, default_value_t = 115200)]
        serial_baud: u32,
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
        Commands::Play {
            midi_port,
            serial_port,
            serial_baud,
        } => {
            let midi_messages = midi_live
                .signal_builder(midi_port)?
                .single_channel(0)
                .build();
            let serial_midi_messages =
                MidiLiveSerial::new(serial_port, serial_baud)?
                    .signal_builder()
                    .single_channel(0)
                    .build();
            run(midi_messages, serial_midi_messages)?
        }
    }
    Ok(())
}
