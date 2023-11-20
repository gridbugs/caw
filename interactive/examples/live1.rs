use clap::{Parser, Subcommand};
use currawong_interactive::prelude::*;

#[allow(unused)]
#[derive(Clone)]
struct Effects {
    high_pass_filter_cutoff: Sf64,
    low_pass_filter_cutoff: Sf64,
    low_pass_filter_resonance: Sf64,
    lfo_freq: Sf64,
    lfo_scale: Sf64,
    detune: Sf64,
    echo_scale: Sf64,
    sah_freq: Sf64,
    sah_scale: Sf64,
    attack: Sf64,
    decay: Sf64,
    sustain: Sf64,
    release: Sf64,
    pulse_width: Sf64,
    squareness: Sf64,
}

impl Effects {
    fn new(controllers: &MidiControllerTable, extra_controllers: &MidiControllerTable) -> Self {
        Self {
            high_pass_filter_cutoff: extra_controllers.get_01(35).exp_01(1.0),
            low_pass_filter_cutoff: extra_controllers.get_01(31).inv_01().exp_01(1.0),
            low_pass_filter_resonance: extra_controllers.get_01(32).exp_01(1.0),
            lfo_freq: extra_controllers.get_01(33),
            lfo_scale: extra_controllers.get_01(34),
            detune: extra_controllers.get_01(36),
            echo_scale: controllers.modulation(),
            sah_freq: controllers.get_01(21),
            sah_scale: controllers.get_01(22),
            attack: controllers.get_01(25),
            decay: controllers.get_01(26),
            sustain: controllers.get_01(27),
            release: extra_controllers.get_01(35),
            pulse_width: controllers.get_01(23),
            squareness: controllers.get_01(24),
        }
    }
}

fn make_voice(
    MidiVoice {
        note_freq,
        velocity_01: _,
        gate,
        ..
    }: MidiVoice,
    pitch_bend_multiplier_hz: impl Into<Sf64>,
    effects: Effects,
) -> Sf64 {
    let velocity_01 = 1.0; // velocity_01.exp_01(-2.0);
    let Effects {
        high_pass_filter_cutoff,
        low_pass_filter_cutoff,
        low_pass_filter_resonance,
        lfo_freq,
        lfo_scale,
        detune,
        echo_scale,
        sah_freq,
        sah_scale,
        attack,
        decay,
        sustain,
        release,
        pulse_width,
        squareness: _,
    } = effects;
    let env = adsr_linear_01(&gate)
        .attack_s(attack)
        .decay_s(decay)
        .sustain_01(sustain)
        .release_s(release.clone())
        .build()
        .exp_01(1.0);
    let note_freq_hz = sfreq_to_hz(note_freq) * pitch_bend_multiplier_hz.into();
    let detune = detune * 0.01;
    let mkosc = |note_freq_hz| {
        sum([
            oscillator_hz(Waveform::Saw, &note_freq_hz).build(),
            oscillator_hz(Waveform::Saw, &note_freq_hz * (1.0 + &detune)).build(),
            oscillator_hz(Waveform::Saw, &note_freq_hz * (1.0 - &detune)).build(),
        ])
    };
    let osc = mkosc(note_freq_hz.clone());
    let amp_env = adsr_linear_01(&gate)
        .attack_s(0.05)
        .release_s(release * 20)
        .build()
        .exp_01(1.0);
    let filtered_osc = osc;
    filtered_osc.mul_lazy(&amp_env)
}

fn freq_hz_by_gate() -> Vec<(Key, f64)> {
    use Key::*;
    let top_row_base = Note::new(NoteName::C, 2).to_midi_index();
    let top_row = vec![W, N3, E, N4, R, T, N6, Y, N7, U, N8, I, O];
    top_row
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, freq_hz_of_midi_index(i as u8 + top_row_base)))
        .collect::<Vec<_>>()
}

fn single_voice(
    note_freq_hz: f64,
    gate: Gate,
    effect_x: Sf64,
    effect_y: Sf64,
    effects: Effects,
) -> Sf64 {
    let velocity_01 = 1.0; // velocity_01.exp_01(-2.0);
    let Effects {
        high_pass_filter_cutoff,
        low_pass_filter_cutoff,
        low_pass_filter_resonance,
        lfo_freq,
        lfo_scale,
        detune,
        echo_scale,
        sah_freq,
        sah_scale,
        attack,
        decay,
        sustain,
        release,
        pulse_width,
        squareness: _,
    } = effects;
    let env = adsr_linear_01(&gate)
        .attack_s(attack)
        .decay_s(decay)
        .sustain_01(sustain)
        .release_s(release.clone())
        .build()
        .exp_01(1.0);
    let detune = detune * 0.01;
    let mkosc = |note_freq_hz| {
        sum([
            oscillator_hz(Waveform::Saw, note_freq_hz).build(),
            oscillator_hz(Waveform::Saw, note_freq_hz * (1.0 + &detune)).build(),
            oscillator_hz(Waveform::Saw, note_freq_hz * (1.0 - &detune)).build(),
        ])
    };
    let osc = mkosc(note_freq_hz) + mkosc(note_freq_hz / 2.0);
    let amp_env = adsr_linear_01(&gate)
        .attack_s(0.05)
        .release_s(release * 20)
        .build()
        .exp_01(1.0);
    let filtered_osc = osc.filter(
        low_pass_moog_ladder(sum([low_pass_filter_cutoff * 20000.0 * amp_env]))
            .resonance(low_pass_filter_resonance * 2.0)
            .build(),
    );
    filtered_osc * lfo_scale * 2.0
}

fn voice(input: Input, effects: Effects) -> Sf64 {
    freq_hz_by_gate()
        .into_iter()
        .map(|(key, freq_hz)| {
            single_voice(
                freq_hz,
                input.key(key),
                input.x_01().clone(),
                input.y_01().clone(),
                effects.clone(),
            )
        })
        .sum::<Sf64>()
}

fn run(signal: Sf64, effects: Effects) -> anyhow::Result<()> {
    let window = Window::builder()
        .scale(10.0)
        .stable(true)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = voice(window.input(), effects) + signal;
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
            let MidiPlayer { channels } = midi_live.into_player(midi_port, 8).unwrap();
            let MidiChannel {
                voices,
                pitch_bend_multiplier_hz,
                controllers,
                ..
            } = channels[0].clone();
            let MidiChannel {
                controllers: serial_controllers,
                ..
            } = MidiLiveSerial::new(serial_port, serial_baud)?.into_player_single_channel(0, 0);
            let effects = Effects::new(&controllers, &serial_controllers);
            let signal = voices
                .into_iter()
                .map(|voice| make_voice(voice, &pitch_bend_multiplier_hz, effects.clone()))
                .sum::<Sf64>();
            run(signal, effects.clone())?;
        }
    }
    Ok(())
}
