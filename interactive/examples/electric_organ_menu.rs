use caw::prelude::*;

struct Effects<DV, DLPF>
where
    DV: SigT<Item = f32>,
    DLPF: SigT<Item = f32>,
{
    drum_volume: DV,
    drum_low_pass_filter: DLPF,
}

impl Effects<f32, f32> {
    fn new() -> Self {
        Self {
            drum_volume: 0.3,
            drum_low_pass_filter: 0.3,
        }
    }
}

fn drum_loop(
    trig: impl FrameSigT<Item = bool>,
    pattern: Vec<u8>,
    channel: Channel,
) -> Sig<impl SigT<Item = f32>> {
    let [trig_hat_closed, trig_snare, trig_kick, ..] =
        bitwise_pattern_trigs_8(trig, pattern);
    let phase_offset_01 = channel.circle_phase_offset_01();
    trig_kick.trig(drum::kick().phase_offset_01(phase_offset_01))
        + trig_snare.trig(drum::snare().phase_offset_01(phase_offset_01))
        + trig_hat_closed.trig(drum::hat_closed())
}

fn voice(
    MonoVoice {
        note,
        key_down_gate,
        key_press_trig,
        ..
    }: MonoVoice<impl FrameSigT<Item = KeyEvents>>,
    effect_x: impl SigT<Item = f32>,
    effect_y: impl SigT<Item = f32>,
    channel: Channel,
) -> Sig<impl SigT<Item = f32>> {
    let note = note.shared();
    let oscillator = oscillator(Saw, note.clone().freq_hz())
        .reset_offset_01(channel.circle_phase_offset_01())
        .build()
        + oscillator(Saw, note.clone().freq_hz() / 2.0)
            .reset_offset_01(channel.circle_phase_offset_01())
            .build();
    let env = adsr_linear_01(key_down_gate)
        .key_press_trig(key_press_trig)
        .attack_s(0.0)
        .decay_s(0.9)
        .sustain_01(0.1)
        .release_s(0.0)
        .build()
        .exp_01(1.0);
    oscillator.filter(
        low_pass::default(env * note.clone().freq_hz() * 30.0 * Sig(effect_x))
            .resonance(Sig(effect_y)),
    )
}

fn voice_bass(
    MonoVoice {
        note,
        key_down_gate,
        key_press_trig,
        ..
    }: MonoVoice<impl FrameSigT<Item = KeyEvents>>,
    channel: Channel,
) -> Sig<impl SigT<Item = f32>> {
    let oscillator = oscillator(Saw, note.freq_hz())
        .reset_offset_01(channel.circle_phase_offset_01())
        .build();
    let env = adsr_linear_01(key_down_gate)
        .key_press_trig(key_press_trig)
        .attack_s(0.1)
        .sustain_01(1.0)
        .release_s(0.1)
        .build()
        .exp_01(1.0);
    oscillator.filter(low_pass::default(env * 1000.0))
}

fn arp_shape(
    trig: impl FrameSigT<Item = bool>,
) -> FrameSig<impl FrameSigT<Item = ArpShape>> {
    use rand::{rngs::StdRng, Rng, SeedableRng};
    struct State {
        rng: StdRng,
        indices: Vec<Option<usize>>,
    }
    let mut state = State {
        rng: StdRng::from_entropy(),
        indices: vec![Some(0); 8],
    };
    const MAX_INDEX: usize = 6;
    let trig = FrameSig(trig);
    trig.divide(16).map(move |trig| {
        if trig {
            let index_to_change =
                state.rng.gen::<usize>() % state.indices.len();
            let value =
                if state.indices.len() <= 1 || state.rng.gen::<f64>() < 0.9 {
                    Some(state.rng.gen::<usize>() % MAX_INDEX)
                } else {
                    None
                };
            state.indices[index_to_change] = value;
        }
        ArpShape::Indices(state.indices.clone())
    })
}

fn virtual_key_events(
    trig: impl FrameSigT<Item = bool>,
) -> FrameSig<impl FrameSigT<Item = KeyEvents>> {
    let chords = [
        chord(note_name::C, MINOR),
        chord(note_name::C, MINOR),
        chord(note_name::G, MINOR),
    ];
    struct State {
        index: usize,
    }
    let mut state = State { index: 0 };
    let trig = FrameSig(trig);
    trig.divide(32).map(move |t| {
        let octave_base = note::A2;
        let mut events = KeyEvents::empty();
        if t {
            if state.index > 0 {
                let prev_chord = chords[(state.index - 1) % chords.len()];
                prev_chord.with_notes(
                    Inversion::InOctave { octave_base },
                    |note| {
                        events.push(KeyEvent {
                            note,
                            pressed: false,
                            velocity_01: 1.0,
                        })
                    },
                )
            }
            let current_chord = chords[state.index % chords.len()];
            current_chord.with_notes(
                Inversion::InOctave { octave_base },
                |note| {
                    events.push(KeyEvent {
                        note,
                        pressed: true,
                        velocity_01: 1.0,
                    })
                },
            );
            state.index += 1;
        }
        events
    })
}

fn virtual_key_events_bass(
    trig: impl FrameSigT<Item = bool>,
) -> FrameSig<impl FrameSigT<Item = KeyEvents>> {
    let notes = [note::C2, note::C2, note::G2];
    struct State {
        index: usize,
    }
    let mut state = State { index: 0 };
    let trig = FrameSig(trig);
    trig.divide(32).map(move |t| {
        let mut events = KeyEvents::empty();
        if t {
            if state.index > 0 {
                let prev_note = notes[(state.index - 1) % notes.len()];
                events.push(KeyEvent {
                    note: prev_note,
                    pressed: false,
                    velocity_01: 1.0,
                })
            }
            let current_note = notes[state.index % notes.len()];
            events.push(KeyEvent {
                note: current_note,
                pressed: true,
                velocity_01: 1.0,
            });
            state.index += 1;
        }
        events
    })
}

fn sig(_input: Input, channel: Channel) -> Sig<impl SigT<Item = f32>> {
    let effects = Effects::new();
    let _hat_closed = 1 << 0;
    let snare = 1 << 1;
    let kick = 1 << 2;
    let trigger = periodic_trig_hz(3.0).build().shared();
    let drums =
        drum_loop(trigger.clone().divide(2), vec![kick, snare], channel);
    let arp_config = ArpConfig::default()
        .with_shape(arp_shape(trigger.clone()))
        .with_extend_octaves_high(1);
    let resonance = oscillator(Sine, 1.0 / 60.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01()
        .shared();
    let cutoff = ((oscillator(Sine, 1.0 / 45.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01()
        * 0.5)
        + 0.3)
        .shared();
    let room_size = (oscillator(Sine, 1.0 / 100.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01()
        * 0.5)
        + 0.2;
    let melody = virtual_key_events(trigger.clone())
        .arp(trigger.clone(), arp_config)
        .poly_voices(12)
        .into_iter()
        .map(move |mono_voice| {
            voice(mono_voice, cutoff.clone(), resonance.clone(), channel)
        })
        .sum::<Sig<_>>()
        .filter(delay_s(0.2).feedback_ratio(0.5).mix_01(0.25))
        .filter(reverb::default().room_size(room_size).damping(0.5))
        .filter(high_pass::butterworth(1.0));
    let bass = voice_bass(
        virtual_key_events_bass(trigger.clone()).mono_voice(),
        channel,
    );
    (drums.filter(low_pass::default(effects.drum_low_pass_filter * 20000.0))
        * effects.drum_volume)
        + (melody * 0.5 + bass * 0.2).filter(
            chorus()
                .feedback_ratio(0.5)
                .lfo_offset(ChorusLfoOffset::Interleave(channel)),
        )
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .scale(0.7)
        .stride(2)
        .build();
    let input = window.input();
    window.play_stereo(
        Stereo::new_fn_channel(|ch| sig(input.clone(), ch)),
        Default::default(),
    )
}
