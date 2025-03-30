use caw_core::*;
use caw_interactive::{Input, Visualization, Window};
use caw_keyboard::{IntoNoteFreqHz_, KeyEvents, KeyEventsT_, MonoVoice_, Note};
use caw_midi::*;
use caw_midi_live::*;
use caw_modules::*;
use clap::{Parser, Subcommand};

fn sig(
    input: Input,
    key_events: Sig<impl SigT<Item = KeyEvents>>,
    _pitch_bend_freq_mult: Sig<SigShared<impl SigT<Item = f32>>>,
    controllers: MidiControllers_<impl SigT<Item = MidiMessages>>,
    channel: Channel,
) -> Sig<impl SigT<Item = f32>> {
    input
        .keyboard
        .opinionated_key_events_(Note::B0)
        .merge(Sig(key_events))
        .poly_voices(48)
        .into_iter()
        .map(
            |MonoVoice_ {
                 note,
                 key_down_gate,
                 key_press_trig,
                 velocity_01: _,
             }| {
                let key_down_gate = key_down_gate.shared();
                let key_press_trigger = key_press_trig.shared();
                let filter_env = adsr_linear_01(key_down_gate.clone())
                    .key_press_trig(key_press_trigger.clone())
                    .attack_s(0.2)
                    .release_s(4.)
                    .build();
                let amp_env = adsr_linear_01(key_down_gate.clone())
                    .key_press_trig(key_press_trigger.clone())
                    .attack_s(0.075)
                    .release_s(1.0)
                    .build()
                    .exp_01(1.);
                let osc = super_saw(note.freq_hz())
                    .num_oscillators(2)
                    .init(SuperSawInit::Const(channel.circle_phase_offset_01()))
                    .detune_ratio(0.008)
                    .build();
                osc.filter(
                    low_pass::default(
                        filter_env * 20_000. * controllers.modulation(),
                    )
                    .resonance(0.),
                ) * amp_env
            },
        )
        .sum::<Sig<_>>()
        .filter(
            chorus()
                .num_voices(3)
                .lfo_rate_hz(0.5)
                .delay_s(input.mouse.x_01() * 0.01)
                .depth_s(input.mouse.y_01() * 0.01)
                .lfo_offset(ChorusLfoOffset::Interleave(channel))
                .mix_01(0.5)
                .feedback_ratio(0.5),
        )
        .filter(reverb::default().room_size(0.5).damping(0.5))
        .filter(high_pass::default(1.))
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

fn run(midi_events: Sig<impl SigT<Item = MidiMessages>>) -> anyhow::Result<()> {
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .scale(0.8)
        .build();
    let input = window.input();
    let midi_events = midi_events.shared();
    let key_events = midi_events.clone().key_events().shared();
    let pitch_bend_freq_mult =
        midi_events.clone().pitch_bend_freq_mult().shared();
    let controllers = midi_events.clone().controllers();
    window.play_stereo(
        Stereo::new_fn_channel(|channel| {
            sig(
                input.clone(),
                key_events.clone(),
                pitch_bend_freq_mult.clone(),
                controllers.clone(),
                channel,
            )
        }),
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
            run(connection.channel_(0))?;
        }
    }
    Ok(())
}
