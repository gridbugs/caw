use clap::{Parser, Subcommand};
use currawong_interactive::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct Effects {
    low_pass_filter_cutoff: Sf64,
    low_pass_filter_resonance: Sf64,
    attack: Sf64,
    decay: Sf64,
    sustain: Sf64,
    release: Sf64,
    reverb_gain: Sf64,
    reverb_room_size: Sf64,
    reverb_damping: Sf64,
    drum_volume: Sf64,
    drum_low_pass_filter_cutoff: Sf64,
    arp_tempo: Sf64,
    arp_period_scale: Sf64,
    arp_type: Sf64,
    compress_threshold: Sf64,
    compress_scale: Sf64,
}

impl Effects {
    fn new(controllers: &MidiControllerTable, extra_controllers: &MidiControllerTable) -> Self {
        Self {
            low_pass_filter_cutoff: extra_controllers.get_01(31).inv_01().exp_01(1.0),
            low_pass_filter_resonance: extra_controllers.get_01(32).exp_01(1.0),
            arp_type: extra_controllers.get_01(33),
            compress_threshold: extra_controllers.get_01(34),
            reverb_gain: extra_controllers.get_01(35),
            reverb_room_size: extra_controllers.get_01(36),
            reverb_damping: extra_controllers.get_01(37),
            compress_scale: extra_controllers.get_01(38),
            drum_volume: controllers.get_01(21),
            drum_low_pass_filter_cutoff: controllers.get_01(22),
            arp_tempo: controllers.get_01(23).exp_01(1.0),
            arp_period_scale: controllers.get_01(24),
            attack: controllers.get_01(25),
            decay: controllers.get_01(26),
            sustain: controllers.get_01(27),
            release: controllers.get_01(28),
        }
    }
}

fn make_voice(note_freq: Sfreq, gate: Gate, effects: &Effects) -> Sf64 {
    let Effects {
        low_pass_filter_cutoff,
        low_pass_filter_resonance,
        attack,
        decay,
        sustain,
        release,
        ..
    } = effects;
    let freq_hz = sfreq_to_hz(note_freq);
    let osc = oscillator_hz(Waveform::Saw, &freq_hz).build()
        + oscillator_hz(Waveform::Saw, &freq_hz * 1.01).build()
        + oscillator_hz(Waveform::Saw, &freq_hz * 0.99).build()
        + oscillator_hz(Waveform::Pulse, &freq_hz * 1.0).build()
        + oscillator_hz(Waveform::Pulse, &freq_hz * 2.0).build();
    let env = adsr_linear_01(&gate)
        .attack_s(attack * 4.0)
        .decay_s(decay * 4.0)
        .sustain_01(sustain)
        .release_s(release * 4.0)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    let filtered_osc = osc.filter(
        low_pass_moog_ladder(sum([&env * (low_pass_filter_cutoff * 20_000.0)]))
            .resonance(low_pass_filter_resonance * 4.0)
            .build(),
    );
    filtered_osc.mul_lazy(&env).filter(
        compress()
            .threshold(&effects.compress_threshold * 2.0)
            .scale(&effects.compress_scale * 8.0)
            .build(),
    )
}

fn kick(trigger: Trigger) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.05;
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 200.0
        + 40.0;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz).build();
    let env_amp = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    let x = osc.filter(low_pass_moog_ladder(400.0).build()) * 8.0;
    let x = (osc + x)
        .filter(low_pass_moog_ladder(500.0 * env_amp).build())
        .filter(compress().ratio(0.02).scale(16.0).build());
    let reverb = x
        .filter(low_pass_moog_ladder(400.0).build())
        .filter(reverb().room_size(0.5).damping(0.5).build());
    let reverb_gate = trigger.to_gate_with_duration_s(0.2);
    let reverb_env = adsr_linear_01(reverb_gate)
        .release_s(0.1)
        .build()
        .exp_01(1.0);
    x + (reverb * reverb_env)
}

fn snare(trigger: Trigger) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.1;
    let env = adsr_linear_01(&clock)
        .release_s(duration_s * 1.0)
        .build()
        .filter(low_pass_moog_ladder(1000.0).build());
    let noise = noise();
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 100.0
        + 40.0;
    let x = oscillator_hz(Waveform::Pulse, freq_hz)
        .reset_trigger(&trigger)
        .build()
        + noise;
    let x = &x + &x.filter(low_pass_moog_ladder(200.0).build()) * 6.0;
    let x = x.mul_lazy(&env);
    let reverb = x.filter(reverb().room_size(0.5).damping(0.5).build());
    let reverb_gate = trigger.to_gate_with_duration_s(0.2);
    let reverb_env = adsr_linear_01(reverb_gate)
        .release_s(0.05)
        .build()
        .exp_01(1.0);
    (x + (reverb * reverb_env)) * 0.5
}

fn cymbal(trigger: Trigger) -> Sf64 {
    let gate = trigger.to_gate();
    let osc = noise();
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_butterworth(100.0).build());
    osc.filter(low_pass_moog_ladder(10000 * &env).build())
        .filter(high_pass_butterworth(6000.0).build())
        * 4.0
}

fn make_drum_voice(
    MidiMonophonic {
        note_index, gate, ..
    }: MidiMonophonic,
    effects: &Effects,
) -> Sf64 {
    let trigger = gate.to_trigger_rising_edge();
    let kick_trigger = trigger.and_fn_ctx({
        let note_index = note_index.clone();
        move |ctx| note_index.sample(ctx) == 36
    });
    let snare_trigger = trigger.and_fn_ctx({
        let note_index = note_index.clone();
        move |ctx| note_index.sample(ctx) == 37
    });
    let cymbal_trigger = trigger.and_fn_ctx({
        let note_index = note_index.clone();
        move |ctx| note_index.sample(ctx) == 38
    });
    (kick(kick_trigger) + snare(snare_trigger) + cymbal(cymbal_trigger))
        .filter(low_pass_moog_ladder(&effects.drum_low_pass_filter_cutoff * 20_000).build())
        * &effects.drum_volume
}

fn make_synth_voice(freq: Sfreq, gate: Gate, effects: &Effects) -> Sf64 {
    let dry = make_voice(freq, gate, effects);
    let reverb = (&dry * &effects.reverb_gain).filter(
        reverb()
            .room_size(&effects.reverb_room_size)
            .damping(&effects.reverb_damping)
            .build(),
    );
    dry + (reverb * 4.0)
}

fn run(signal: Sf64, _effects: &Effects) -> anyhow::Result<()> {
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

struct SortedVec<T> {
    vec: Vec<T>,
}

impl<T> Default for SortedVec<T> {
    fn default() -> Self {
        Self {
            vec: Default::default(),
        }
    }
}

impl<T: PartialOrd> SortedVec<T> {
    fn insert(&mut self, element: T) {
        let mut i = 0;
        for x in &self.vec {
            if &element == x {
                return;
            }
            if &element < x {
                break;
            }
            i += 1;
        }
        self.vec.insert(i, element);
    }

    fn remove(&mut self, element: T) {
        for (i, x) in self.vec.iter().enumerate() {
            if x == &element {
                self.vec.remove(i);
                break;
            }
        }
    }
}

#[derive(Default)]
struct ArpState {
    pressed_keys: SortedVec<Note>,
    index: usize,
    current_note: Note,
}

fn midi_keyboard_arp(
    messages: Signal<MidiMessages>,
    trigger: Trigger,
    effects: &Effects,
) -> (Sfreq, Gate) {
    let state = Rc::new(RefCell::new(ArpState::default()));
    let rng = RefCell::new(StdRng::from_entropy());
    let (freq, any_pressed) = messages
        .map_ctx({
            let state = Rc::clone(&state);
            let arp_type = effects.arp_type.clone();
            move |messages, ctx| {
                messages.for_each(|message| match message {
                    MidiMessage::NoteOn { key, .. } => {
                        state
                            .borrow_mut()
                            .pressed_keys
                            .insert(Note::from_midi_index(key));
                    }
                    MidiMessage::NoteOff { key, .. } => {
                        state
                            .borrow_mut()
                            .pressed_keys
                            .remove(Note::from_midi_index(key));
                    }
                    _ => (),
                });
                if trigger.sample(ctx) {
                    let mut state = state.borrow_mut();
                    let num_pressed_keys = state.pressed_keys.vec.len();
                    if num_pressed_keys == 0 {
                        state.index = 0;
                    } else {
                        state.current_note = state.pressed_keys.vec[state.index % num_pressed_keys];
                        let arp_type = arp_type.sample(ctx);
                        if arp_type < 0.3 {
                            state.index = (state.index + 1) % num_pressed_keys;
                        } else if arp_type < 0.6 {
                            state.index = (num_pressed_keys + state.index - 1) % num_pressed_keys;
                        } else {
                            let mut rng = rng.borrow_mut();
                            state.index = rng.gen::<usize>() % num_pressed_keys;
                        }
                    }
                }
            }
        })
        .map({
            let state = Rc::clone(&state);
            move |()| {
                let state = state.borrow();
                (
                    state.current_note.freq(),
                    !state.pressed_keys.vec.is_empty(),
                )
            }
        })
        .unzip();
    (freq, any_pressed.to_gate())
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
            let midi_events = midi_live.signal_builder(midi_port)?.build();
            let keyboard_messages = midi_events.filter_channel(0);
            let controllers = keyboard_messages.controller_table();
            let serial_controllers = MidiLiveSerial::new(serial_port, serial_baud)?
                .signal_builder()
                .single_channel(0)
                .build()
                .controller_table();
            let effects = Effects::new(&controllers, &serial_controllers);
            let arp_period_s = 1.0 / (1.0 + (&effects.arp_tempo * 16));
            let arp_trigger = periodic_trigger_s(&arp_period_s).build();
            let (synth_freq, arp_gate) =
                midi_keyboard_arp(keyboard_messages.clone(), arp_trigger.clone(), &effects);
            let synth_gate = arp_trigger
                .to_gate_with_duration_s(arp_period_s * &effects.arp_period_scale)
                .and(arp_gate);
            let synth_signal = make_synth_voice(synth_freq, synth_gate, &effects);
            let drum_signal = make_drum_voice(midi_events.filter_channel(9).monophonic(), &effects);
            run(synth_signal + drum_signal, &effects)?;
        }
    }
    Ok(())
}
