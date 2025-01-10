use caw::prelude::*;
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};

struct Effects<T, DV, DLPF>
where
    T: SigT<Item = f32>,
    DV: SigT<Item = f32>,
    DLPF: SigT<Item = f32>,
{
    tempo: T,
    drum_volume: DV,
    drum_low_pass_filter: DLPF,
}

impl Effects<f32, f32, f32> {
    fn new() -> Self {
        Self {
            tempo: 0.6,
            drum_volume: 0.4,
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
    effect_z: impl SigT<Item = f32>,
    channel: Channel,
) -> Sig<impl SigT<Item = f32>> {
    let note = note.shared();
    let lfo = oscillator(Sine, 8.0).build().signed_to_01();
    let oscillator = oscillator(Pulse, note.clone().freq_hz())
        .pulse_width_01(effect_z)
        .reset_offset_01(channel.circle_phase_offset_01())
        .build();
    let env = adsr_linear_01(key_down_gate)
        .key_press_trig(key_press_trig)
        .attack_s(0.0)
        .decay_s(1.0)
        .sustain_01(0.1)
        .release_s(0.0)
        .build()
        .exp_01(1.0);
    oscillator.filter(low_pass::default(
        (env * Sig(effect_x) * 10000.0) + (lfo * 2000.0 * Sig(effect_y)),
    ))
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
    let oscillator = oscillator(Triangle, note.freq_hz() / 2.0)
        .reset_offset_01(channel.circle_phase_offset_01())
        .build();
    let env = adsr_linear_01(key_down_gate)
        .key_press_trig(key_press_trig)
        .attack_s(0.0)
        .release_s(0.1)
        .build()
        .exp_01(1.0);
    oscillator.filter(low_pass::default(5000.0)) * env
}

fn virtual_key_events(
    trig: impl FrameSigT<Item = bool>,
) -> FrameSig<impl FrameSigT<Item = KeyEvents>> {
    let chords = [
        chord(note_name::C, MINOR),
        chord(note_name::E, MAJOR),
        chord(note_name::F, MAJOR),
        chord(note_name::F, MINOR),
    ];
    struct State {
        index: usize,
    }
    let mut state = State { index: 0 };
    let trig = FrameSig(trig);
    trig.divide(32).map(move |t| {
        let inversion = Inversion::InOctave {
            octave_base: note::A2,
        };
        let mut events = KeyEvents::empty();
        if t {
            if state.index > 0 {
                let prev_chord = chords[(state.index - 1) % chords.len()];
                let mut count = 0;
                prev_chord.with_notes(inversion, |note| {
                    events.push(KeyEvent {
                        note,
                        pressed: false,
                        velocity_01: 1.0,
                    });
                    if count < 2 {
                        events.push(KeyEvent {
                            note: note.add_octaves(1),
                            pressed: false,
                            velocity_01: 1.0,
                        });
                    }
                    count += 1;
                })
            }
            let mut count = 0;
            let current_chord = chords[state.index % chords.len()];
            current_chord.with_notes(inversion, |note| {
                events.push(KeyEvent {
                    note,
                    pressed: true,
                    velocity_01: 1.0,
                });
                if count < 2 {
                    events.push(KeyEvent {
                        note: note.add_octaves(1),
                        pressed: true,
                        velocity_01: 1.0,
                    });
                }
                count += 1;
            });
            state.index += 1;
        }
        events
    })
}

fn virtual_key_events_bass(
    trig: impl FrameSigT<Item = bool>,
) -> FrameSig<impl FrameSigT<Item = KeyEvents>> {
    let notes = [note::C2, note::E2, note::F2, note::F2];
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

fn sig(
    _input: Input,
    channel: Channel,
    seed: u64,
) -> Sig<impl SigT<Item = f32>> {
    let effects = Effects::new();
    let _hat_closed = 1 << 0;
    let snare = 1 << 1;
    let kick = 1 << 2;
    let trigger = periodic_trig_hz(effects.tempo * 8.).build().shared();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut snares = (0..11).map(|_| 0).collect::<Vec<_>>();
    for _ in 0..4 {
        snares.push(snare);
    }
    snares.shuffle(&mut rng);
    let snares = drum_loop(trigger.clone(), snares, channel);
    #[rustfmt::skip]
    let drums = drum_loop(
        trigger.clone(),
        vec![
            kick,
            0,
            0,
            0,
            kick,
            0,
            0,
            0,
        ],
        channel,
    ) + snares;
    let arp_config = ArpConfig::default()
        .with_shape(ArpShape::Down)
        .with_extend_octaves_high(0);
    let distortion = oscillator(Sine, 1.0 / 89.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01()
        .shared();
    let resonance = (oscillator(Sine, 1.0 / 127.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01()
        * 0.3)
        .shared();
    let cutoff = (oscillator(Sine, 1.0 / 53.0)
        .reset_offset_01(-0.25)
        .build()
        .signed_to_01()
        * 0.3
        + 0.1)
        .shared();
    let effect_z =
        (oscillator(Sine, 97.0).build().signed_to_01() * 0.3 + 0.2).shared();
    let keys = virtual_key_events(trigger.clone())
        .arp(trigger.clone(), arp_config)
        .poly_voices(1)
        .into_iter()
        .map(|mono_voice| {
            voice(
                mono_voice,
                cutoff.clone(),
                resonance.clone(),
                effect_z.clone(),
                channel,
            )
        })
        .sum::<Sig<_>>()
        .filter(
            compressor()
                .scale(1.0 + distortion.clone() * 4.0)
                .threshold(1.0 - distortion.clone() * 0.5),
        )
        .filter(reverb::default().room_size(0.9).damping(0.5))
        .filter(high_pass::butterworth(1.0));
    let bass = voice_bass(
        virtual_key_events_bass(trigger.clone()).mono_voice(),
        channel,
    );
    (drums.filter(low_pass::default(effects.drum_low_pass_filter * 20000.0))
        * effects.drum_volume)
        + (keys * (0.6 - cutoff))
        + bass * 0.2
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
    let rng_seed = StdRng::from_entropy().gen::<u64>();
    window.play_stereo(
        Stereo::new_fn_channel(|ch| sig(input.clone(), ch, rng_seed)),
        Default::default(),
    )
}
