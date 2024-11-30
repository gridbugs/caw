use caw_core_next::*;
use caw_interactive_next::{Input, MouseButton, Visualization, Window};
use caw_keyboard::{IntoNoteFreqHz, KeyEvents, KeyEventsT, MonoVoice, Note};
use caw_midi::*;
use caw_midi_live::*;
use caw_modules::*;
use clap::{Parser, Subcommand};

fn sig(
    input: Input,
    key_events: FrameSig<impl FrameSigT<Item = KeyEvents>>,
    pitch_bend_freq_mult: FrameSig<FrameSigShared<impl FrameSigT<Item = f32>>>,
) -> Sig<impl SigT<Item = f32>> {
    input
        .keyboard
        .opinionated_key_events(Note::B0)
        .merge(FrameSig(key_events))
        .poly_voices(12)
        .into_iter()
        .map(
            |MonoVoice {
                 note,
                 key_down_gate,
                 key_press_trigger,
                 ..
             }| {
                let key_down_gate =
                    key_down_gate | input.mouse.button(MouseButton::Left);
                let env = adsr_linear_01(key_down_gate)
                    .key_press_trig(key_press_trigger)
                    .attack_s(0.01)
                    .decay_s(0.2)
                    .sustain_01(0.5)
                    .release_s(0.1)
                    .build()
                    .exp_01(1);
                let osc =
                    super_saw(note.freq_hz() * pitch_bend_freq_mult.clone())
                        .num_oscillators(32)
                        .build();
                osc.filter(
                    low_pass::default(env * input.mouse.y_01() * 10000)
                        .resonance(input.mouse.x_01()),
                )
            },
        )
        .sum::<Sig<_>>()
        .filter(reverb::default().room_size(0.9).damping(0.9))
        .filter(high_pass::default(1))
        * 0.25
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
    },
}

fn run(
    midi_events: FrameSig<impl FrameSigT<Item = MidiMessages>>,
) -> anyhow::Result<()> {
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .build();
    let input = window.input();
    let midi_events = midi_events.shared();
    let key_events = midi_events.clone().key_events().shared();
    let pitch_bend_freq_mult =
        midi_events.clone().pitch_bend_freq_mult().shared();
    window.play_stereo(
        sig(
            input.clone(),
            key_events.clone(),
            pitch_bend_freq_mult.clone(),
        ),
        sig(
            input.clone(),
            key_events.clone(),
            pitch_bend_freq_mult.clone(),
        ),
        Default::default(),
    )
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
            let connection = midi_live.connect(midi_port)?;
            run(connection.channel(0))?;
        }
    }
    Ok(())
}
