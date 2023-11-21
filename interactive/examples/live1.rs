use clap::{Parser, Subcommand};
use currawong_interactive::prelude::*;

#[allow(unused)]
#[derive(Clone)]
struct Effects {
    bass_low_pass_filter_cutoff: Sf64,
    treble_low_pass_filter_cutoff: Sf64,
    low_pass_filter_resonance: Sf64,
    bass_volume: Sf64,
    detune: Sf64,
    release_bass: Sf64,
    release_treble: Sf64,
    sustain_bass: Sf64,
    bitcrush: Sf64,
}

impl Effects {
    fn new(controllers: &MidiControllerTable, extra_controllers: &MidiControllerTable) -> Self {
        Self {
            bass_low_pass_filter_cutoff: extra_controllers.get_01(31).inv_01().exp_01(1.0),
            treble_low_pass_filter_cutoff: controllers.modulation().inv_01().exp_01(1.0),
            low_pass_filter_resonance: extra_controllers.get_01(32).exp_01(1.0),
            bitcrush: extra_controllers.get_01(37),
            bass_volume: extra_controllers.get_01(34),
            detune: extra_controllers.get_01(36),
            release_bass: extra_controllers.get_01(33),
            release_treble: extra_controllers.get_01(35),
            sustain_bass: extra_controllers.get_01(38),
        }
    }
}

fn super_saw_osc(freq_hz: Sf64, detune: Sf64, n: usize, gate: Gate) -> Sf64 {
    let trigger = gate.to_trigger_rising_edge();
    (0..n)
        .map(|i| {
            let delta_hz = ((i + 1) as f64 * &detune) / n as f64;
            let osc = oscillator_hz(Waveform::Saw, &freq_hz * (1 + &delta_hz))
                .reset_offset_01(noise_01())
                .reset_trigger(&trigger)
                .build()
                + oscillator_hz(Waveform::Saw, &freq_hz * (1 - &delta_hz))
                    .reset_offset_01(noise_01())
                    .reset_trigger(&trigger)
                    .build();
            osc / (i as f64 + 1.0)
        })
        .sum::<Sf64>()
        + oscillator_hz(Waveform::Saw, &freq_hz).build()
}

fn treble_voice(
    MidiVoice {
        note_freq,
        velocity_01,
        gate,
        ..
    }: MidiVoice,
    pitch_bend_multiplier_hz: impl Into<Sf64>,
    effects: Effects,
) -> Sf64 {
    let velocity_01 = velocity_01.exp_01(-2.0);
    let Effects {
        treble_low_pass_filter_cutoff,
        low_pass_filter_resonance,
        detune,
        release_treble,
        bitcrush,
        ..
    } = effects;
    let note_freq_hz = sfreq_to_hz(note_freq) * pitch_bend_multiplier_hz.into();
    let mkosc = |note_freq_hz| super_saw_osc(note_freq_hz, detune * 0.02, 7, gate.clone());
    let osc = mkosc(note_freq_hz.clone());
    let env = adsr_linear_01(&gate)
        .release_s(0.1 + release_treble * 20)
        .build()
        .exp_01(-1.0);
    let filtered_osc = osc.filter(
        low_pass_moog_ladder(&treble_low_pass_filter_cutoff * 20000)
            .resonance(low_pass_filter_resonance * 2.0)
            .build(),
    );
    (filtered_osc * velocity_01 * &env * 2.0)
        .both(&bitcrush)
        .map(|(x, bitcrush)| {
            if bitcrush == 0.0 {
                x
            } else {
                let scale = 16.0 - (bitcrush * 15.0);
                let scaled_sample = (x * scale) as i64;
                scaled_sample as f64 / scale as f64
            }
        })
        .lazy_zero(&env)
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

fn bass_voice(
    note_freq_hz: f64,
    gate: Gate,
    _effect_x: Sf64,
    _effect_y: Sf64,
    effects: Effects,
) -> Sf64 {
    let Effects {
        bass_low_pass_filter_cutoff,
        low_pass_filter_resonance,
        bass_volume,
        detune,
        release_bass,
        sustain_bass,
        ..
    } = effects;
    let detune = detune * 0.01;
    let mkosc = |note_freq_hz| {
        sum([
            oscillator_hz(Waveform::Saw, note_freq_hz).build(),
            oscillator_hz(Waveform::Saw, note_freq_hz * (1.0 + &detune)).build(),
            oscillator_hz(Waveform::Saw, note_freq_hz * (1.0 - &detune)).build(),
        ])
    };
    let osc = mkosc(note_freq_hz) + mkosc(note_freq_hz / 2.0);
    let env = adsr_linear_01(&gate)
        .decay_s(0.1)
        .sustain_01(sustain_bass)
        .release_s(0.2 + &release_bass * 20)
        .build()
        .exp_01(1.0);
    let amp_env = adsr_linear_01(&gate)
        .release_s(0.2 + &release_bass * 20)
        .build()
        .exp_01(1.0);
    let filtered_osc = osc.filter(
        low_pass_moog_ladder(sum([bass_low_pass_filter_cutoff * 20000.0 * &env]))
            .resonance(low_pass_filter_resonance * 2.0)
            .build(),
    );
    (filtered_osc * bass_volume * 2.0).mul_lazy(&amp_env)
}

fn voice(input: Input, effects: Effects) -> Sf64 {
    freq_hz_by_gate()
        .into_iter()
        .map(|(key, freq_hz)| {
            bass_voice(
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
    let signal = voice(window.input(), effects.clone()) + signal;
    let signal = signal.filter(high_pass_butterworth(20.0).build());
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
            let MidiPlayer { channels } = midi_live.into_player(midi_port, 4).unwrap();
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
                .map(|voice| treble_voice(voice, &pitch_bend_multiplier_hz, effects.clone()))
                .sum::<Sf64>();
            run(signal, effects.clone())?;
        }
    }
    Ok(())
}
