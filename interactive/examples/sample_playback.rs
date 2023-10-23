use clap::{Parser, Subcommand};
use currawong_interactive::prelude::*;
use std::path::PathBuf;

fn make_voice(
    MidiVoice {
        note_freq,
        velocity_01: _,
        gate,
        ..
    }: MidiVoice,
    pitch_bend_multiplier_hz: &Sf64,
    _controllers: &MidiControllerTable,
    _serial_controllers: &MidiControllerTable,
) -> Sf64 {
    let note_freq_hz = note_freq.hz() * pitch_bend_multiplier_hz;
    let osc = oscillator_hz(Waveform::Saw, note_freq_hz).build();
    let env = adsr_linear_01(gate).build();
    osc * env
}

fn make_pad_voice(
    samples: &[Sample],
    MidiVoice {
        note_index, gate, ..
    }: MidiVoice,
    serial_controllers: &MidiControllerTable,
) -> Sf64 {
    let filter_cutoff = serial_controllers.get_01(31) * 4000.0;
    let filter_resonance = serial_controllers.get_01(32) * 8.0;
    let lfo = oscillator_hz(Waveform::Saw, serial_controllers.get_01(33) * 20.0)
        .build()
        .signed_to_01();
    let lfo_scale = serial_controllers.get_01(34) * 1000.0;
    let base_note_index = 36;
    let trigger = gate.to_trigger_rising_edge();
    let samples = samples
        .into_iter()
        .enumerate()
        .map(move |(i, sample)| {
            let target_note_index = base_note_index + i;
            let note_index = note_index.clone();
            let trigger =
                trigger.and_fn_ctx(move |ctx| note_index.sample(ctx) as usize == target_note_index);
            sampler(sample).trigger(trigger.clone()).build()
        })
        .sum::<Sf64>();
    samples.filter(
        low_pass_moog_ladder((filter_cutoff + lfo * lfo_scale).clamp_non_negative())
            .resonance(filter_resonance)
            .build(),
    )
}

fn run(signal: Sf64) -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(10.0)
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
        #[arg(short, long)]
        serial_port: String,
        #[arg(long, default_value_t = 115200)]
        serial_baud: u32,
        #[arg(long)]
        samples: Vec<PathBuf>,
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
            samples,
        } => {
            let MidiPlayer { channels } = midi_live.into_player(midi_port, 8).unwrap();
            let MidiChannel {
                controllers: serial_controllers,
                ..
            } = MidiLiveSerial::new(serial_port, serial_baud)?.into_player_single_channel(0, 0);
            let volume = channels[0].controllers.volume();
            let pitch_bend_multiplier_hz = channels[0].pitch_bend_multiplier_hz.clone();
            let controllers = channels[0].controllers.clone();
            let synth_signal = channels[0]
                .voices
                .iter()
                .map(|voice| {
                    make_voice(
                        voice.clone(),
                        &pitch_bend_multiplier_hz,
                        &controllers,
                        &serial_controllers,
                    )
                })
                .sum::<Sf64>();
            let samples = samples
                .into_iter()
                .map(|path| read_wav(path).unwrap())
                .collect::<Vec<_>>();
            let pad_signal = channels[9]
                .voices
                .iter()
                .map(|voice| make_pad_voice(&samples, voice.clone(), &serial_controllers))
                .sum::<Sf64>();
            let signal = sum([synth_signal, pad_signal]).filter(
                saturate()
                    .scale((serial_controllers.get_01(37) * 9.0) + 1.0)
                    .threshold((serial_controllers.get_01(38).inv_01() * 3.9) + 0.1)
                    .build(),
            ) * volume;
            run(signal)?;
        }
    }
    Ok(())
}
