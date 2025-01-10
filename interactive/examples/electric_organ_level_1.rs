use caw_core::*;
use caw_interactive::{Input, Visualization, Window};
use caw_keyboard::{chord::*, *};
use caw_modules::*;
use caw_patches::{drum, pulse_pwm};
use caw_utils::bitwise_pattern_trigs_8;
use std::cmp::Ordering;

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
            tempo: 0.8,
            drum_volume: 0.8,
            drum_low_pass_filter: 0.5,
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
    let oscillator = super_saw(note.clone().freq_hz())
        .num_oscillators(2)
        .init(SuperSawInit::Const(channel.circle_phase_offset_01()))
        .build();
    let env = adsr_linear_01(key_down_gate)
        .key_press_trig(key_press_trig)
        .attack_s(0.0)
        .decay_s(1.0)
        .sustain_01(0.1)
        .release_s(0.0)
        .build()
        .exp_01(1.0);
    oscillator.filter(
        low_pass::default(env * note.clone().freq_hz() * 30.0 * Sig(effect_x))
            .resonance(Sig(effect_y) * 4.0),
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
    let oscillator = pulse_pwm(note.freq_hz() / 2.0)
        .lfo_reset_offset_01(channel.circle_phase_offset_01())
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
        chord(note_name::B, MAJOR),
        chord(note_name::F, MINOR),
        chord(note_name::G, MINOR),
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
    let notes = [note::C2, note::B2, note::F2, note::G2];
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
    let hat_closed = 1 << 0;
    let snare = 1 << 1;
    let kick = 1 << 2;
    let trigger = periodic_trig_hz(effects.tempo * 8.).build().shared();
    let mut drums0 = drum_loop(
        trigger.clone(),
        vec![
            kick, hat_closed, hat_closed, kick, snare, hat_closed, kick,
            hat_closed, hat_closed, hat_closed, hat_closed, kick, snare,
            hat_closed, kick, hat_closed,
        ],
        channel,
    );
    let mut drums1 = drum_loop(
        trigger.clone(),
        vec![
            kick, 0, 0, kick, snare, 0, kick, 0, 0, 0, 0, kick, snare, 0, kick,
            0,
        ],
        channel,
    );
    let mut drums2 = drum_loop(
        trigger.clone(),
        vec![kick, 0, 0, 0, snare, 0, 0, 0, kick, 0, 0, 0, snare, 0, 0, 0],
        channel,
    );
    let drums = {
        let mut count = -1;
        let mut trigger = trigger.clone().divide(128);
        Sig::from_buf_fn(move |ctx, buf| {
            if trigger.frame_sample(ctx) {
                count += 1;
            }
            match (count % 4).cmp(&2) {
                Ordering::Less => drums2.sample(ctx).clone_to_vec(buf),
                Ordering::Equal => drums1.sample(ctx).clone_to_vec(buf),
                Ordering::Greater => drums0.sample(ctx).clone_to_vec(buf),
            }
        })
    };
    let arp_config = ArpConfig::default()
        .with_shape(ArpShape::Random)
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
    let keys = virtual_key_events(trigger.clone())
        .arp(trigger.clone(), arp_config)
        .poly_voices(1)
        .into_iter()
        .map(|mono_voice| {
            voice(mono_voice, cutoff.clone(), resonance.clone(), channel)
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
        + (keys * (0.9 - cutoff)).filter(
            chorus()
                .feedback_ratio(0.7)
                .lfo_offset(ChorusLfoOffset::Interleave(channel)),
        )
        + bass * 0.3
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
