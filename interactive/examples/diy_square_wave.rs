use clap::{Parser, Subcommand};
use currawong_interactive::prelude::*;

#[allow(unused)]
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
            release: controllers.get_01(28),
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
    controllers: &MidiControllerTable,
    serial_controllers: &MidiControllerTable,
) -> Sf64 {
    let velocity_01 = 1.0; // velocity_01.exp_01(-2.0);
    let Effects {
        high_pass_filter_cutoff,
        low_pass_filter_cutoff,
        low_pass_filter_resonance,
        lfo_freq,
        lfo_scale,
        detune: _,
        echo_scale,
        sah_freq,
        sah_scale,
        attack,
        decay,
        sustain,
        release,
        pulse_width,
        squareness: _,
    } = Effects::new(controllers, serial_controllers);
    let env = adsr_linear_01(&gate)
        .attack_s(attack * 10)
        .decay_s(decay * 10)
        .sustain_01(sustain)
        .release_s(release.clone() * 10)
        .build()
        .exp_01(1.0);
    let note_freq_hz = sfreq_to_hz(note_freq) * pitch_bend_multiplier_hz.into();
    let _pulse_width = (oscillator_hz(Waveform::Triangle, pulse_width * 20)
        .build()
        .signed_to_01()
        * 0.2)
        + 0.1;
    let num_sine_waves = 20;
    let sine_waves = (0..num_sine_waves)
        .map(|i| {
            let n = (i * 2) + 1;
            oscillator_hz(Waveform::Sine, &note_freq_hz * n).build() / n
        })
        .collect::<Vec<_>>();
    let osc = env.map_ctx(move |squareness, ctx| {
        let num_to_take = ((squareness * (num_sine_waves - 1) as f64) + 1.0) as usize;
        let mut sum = 0.0;
        for (i, signal) in sine_waves.iter().enumerate() {
            let value = signal.sample(ctx);
            if i < num_to_take {
                sum += value;
            }
        }
        sum
    });
    let sah_clock = periodic_gate_s(sah_freq + 0.01)
        .build()
        .to_trigger_rising_edge();
    let sah = noise()
        .signed_to_01()
        .filter(sample_and_hold(sah_clock.clone()).build());
    let amp_env = adsr_linear_01(&gate)
        .attack_s(0.05)
        .release_s(release * 20)
        .build()
        .exp_01(1.0);
    let lfo_lfo = noise()
        .filter(sample_and_hold(periodic_gate_s(0.5).build().to_trigger_rising_edge()).build())
        .signed_to_01();
    let lfo = (oscillator_hz(Waveform::Saw, lfo_freq * (5.0 + lfo_lfo * 10.0))
        .reset_trigger(gate.to_trigger_rising_edge())
        .build()
        * 1)
    .signed_to_01()
        * lfo_scale;
    let filtered_osc = osc
        .filter(
            low_pass_moog_ladder(sum([
                200.0.into(),
                &low_pass_filter_cutoff * 20_000.0,
                lfo * 10_000.0,
                (sah * sah_scale * 5000),
            ]))
            .resonance(low_pass_filter_resonance * 4.0)
            .build(),
        )
        .filter(high_pass_butterworth(high_pass_filter_cutoff * 4000.0 + 10.0).build());
    (filtered_osc.mul_lazy(&amp_env).force_lazy(&env) * velocity_01)
        .filter(echo().time_s(0.2).scale(echo_scale).build())
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
            let signal = voices
                .into_iter()
                .map(|voice| {
                    make_voice(
                        voice,
                        &pitch_bend_multiplier_hz,
                        &controllers,
                        &serial_controllers,
                    )
                })
                .sum::<Sf64>()
                .filter(
                    saturate()
                        .scale((serial_controllers.get_01(37) * 9.0) + 1.0)
                        .threshold((serial_controllers.get_01(38).inv_01() * 3.9) + 0.1)
                        .build(),
                );
            run(signal)?;
        }
    }
    Ok(())
}
