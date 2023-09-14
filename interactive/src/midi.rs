use crate::{
    music,
    signal::{Freq, Gate, Sf64, Sfreq, Signal, SignalCtx},
};
pub use midly::{
    num::{u14, u4, u7},
    MidiMessage,
};
use midly::{Format, MetaMessage, PitchBend, Smf, Timing, TrackEvent, TrackEventKind};
use std::{cell::RefCell, fs, path::Path, rc::Rc};

#[derive(Debug, Clone, Copy)]
pub struct MidiEvent {
    pub channel: u4,
    pub message: MidiMessage,
}

pub struct MidiFile {
    smf: Smf<'static>,
}

impl MidiFile {
    pub fn read(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let raw = fs::read(path)?;
        let smf = Smf::parse(&raw)?.make_static();
        Ok(Self { smf })
    }

    pub fn format(&self) -> Format {
        self.smf.header.format
    }

    pub fn num_tracks(&self) -> usize {
        self.smf.tracks.len()
    }

    pub fn track(&self, index: usize) -> anyhow::Result<MidiTrack> {
        if let Some(events) = self.smf.tracks.get(index) {
            Ok(MidiTrack::from_midly_track(self.smf.header.timing, events))
        } else {
            anyhow::bail!(
                "Track index {} is out of range (there are {} tracks)",
                index,
                self.num_tracks()
            )
        }
    }
}

#[derive(Debug, Clone)]
struct MidiTrackEntry {
    absolute_time: u32,
    midi_events: Vec<MidiEvent>,
}

#[derive(Debug, Clone)]
pub struct MidiTrack {
    ticks_per_second: f64,
    track_entries: Vec<MidiTrackEntry>,
}

const DEFAULT_US_PER_BEAT: u32 = 250_000;

impl MidiTrack {
    fn from_midly_track<'a>(timing: Timing, track: &[TrackEvent]) -> Self {
        let tempo_us_per_beat = track.iter().find_map(|e| {
            if let TrackEventKind::Meta(MetaMessage::Tempo(tempo_us_per_beat)) = e.kind {
                Some(tempo_us_per_beat.as_int())
            } else {
                None
            }
        });
        let ticks_per_second = match timing {
            Timing::Metrical(ticks_per_beat) => {
                let us_per_beat = if let Some(tempo_us_per_beat) = tempo_us_per_beat {
                    tempo_us_per_beat
                } else {
                    log::warn!(
                        "Unspecified tempo. Assuming {} microseconds per beat.",
                        DEFAULT_US_PER_BEAT
                    );
                    DEFAULT_US_PER_BEAT
                } as f64;
                (ticks_per_beat.as_int() as f64 * 1_000_000.0) / us_per_beat
            }
            Timing::Timecode(frames_per_second, ticks_per_frame) => {
                frames_per_second.as_f32() as f64 * ticks_per_frame as f64
            }
        };
        let mut track_entries: Vec<MidiTrackEntry> = Vec::new();
        let mut absolute_time = 0;
        for event in track {
            if let TrackEventKind::Midi { channel, message } = event.kind {
                if event.delta == 0 {
                    if let Some(prev) = track_entries.last_mut() {
                        // this is not the first event to happen at this time
                        prev.midi_events.push(MidiEvent { channel, message });
                    } else {
                        // this is the very first event and it has a delta of 0
                        track_entries.push(MidiTrackEntry {
                            absolute_time: 0,
                            midi_events: vec![MidiEvent { channel, message }],
                        });
                    }
                } else {
                    // first event of a new time
                    absolute_time += event.delta.as_int();
                    track_entries.push(MidiTrackEntry {
                        absolute_time,
                        midi_events: vec![MidiEvent { channel, message }],
                    });
                }
            }
        }
        Self {
            ticks_per_second,
            track_entries,
        }
    }
}

pub struct MidiVoiceRaw {
    pub note_index: Signal<u7>,
    pub velocity: Signal<u7>,
    pub gate: Gate,
}

pub struct MidiVoice {
    pub note_freq: Sfreq,
    pub velocity_01: Sf64,
    pub gate: Gate,
}

fn signal_u7_to_01(s: &Signal<u7>) -> Sf64 {
    s.map(|x| x.as_int() as f64 / 127.0)
}

impl MidiVoiceRaw {
    pub fn to_voice(&self) -> MidiVoice {
        MidiVoice {
            note_freq: self
                .note_index
                .map(|i| Freq::from_hz(music::freq_hz_of_midi_index(i.as_int()))),
            velocity_01: signal_u7_to_01(&self.velocity),
            gate: self.gate.clone(),
        }
    }
}

pub struct MidiControllerTableRaw {
    values: Vec<Signal<u7>>,
}

pub struct MidiControllerTable {
    values_01: Vec<Sf64>,
}

impl MidiControllerTableRaw {
    pub fn get(&self, i: u7) -> Signal<u7> {
        self.values[i.as_int() as usize].clone()
    }

    pub fn to_controller_table(&self) -> MidiControllerTable {
        MidiControllerTable {
            values_01: self.values.iter().map(signal_u7_to_01).collect(),
        }
    }
}

impl MidiControllerTable {
    pub fn get_01(&self, i: u7) -> Sf64 {
        self.values_01[i.as_int() as usize].clone()
    }
}

pub struct MidiPlayerRaw {
    pub voices: Vec<MidiVoiceRaw>,
    pub controllers: MidiControllerTableRaw,
    pub pitch_bend: Signal<u14>,
}

pub struct MidiPlayer {
    pub voices: Vec<MidiVoice>,
    pub controllers: MidiControllerTable,
    pub pitch_bend_1: Sf64,
    pub pitch_bend_multiplier: Sf64,
}

trait MidiEventSource {
    fn for_each_new_event<F>(&mut self, ctx: &SignalCtx, f: F)
    where
        F: FnMut(MidiEvent);
}

impl MidiPlayerRaw {
    fn new(
        &self,
        channel: u4,
        polyphony: usize,
        mut event_source: impl MidiEventSource + 'static,
    ) -> MidiPlayerRaw {
        struct Voice {
            note_index: u7,
            velocity: u7,
            gate: bool,
        }
        struct State {
            voices: Vec<Option<Voice>>,
            controllers: Vec<u7>,
            pitch_bend: u14,
        }
        let state = Rc::new(RefCell::new(State {
            voices: (0..polyphony).map(|_| None).collect(),
            controllers: (0..128).map(|_| u7::new(0)).collect(),
            pitch_bend: u14::new(0x2000),
        }));
        let effectful_signal = Signal::from_fn({
            let state = Rc::clone(&state);
            move |ctx| {
                let mut state = state.borrow_mut();
                event_source.for_each_new_event(ctx, |event| {
                    if event.channel == channel {
                        use MidiMessage::*;
                        match event.message {
                            NoteOn { key, vel } => (),
                            NoteOff { key, vel } => (),
                            PitchBend {
                                bend: PitchBend(pitch_bend),
                            } => state.pitch_bend = pitch_bend,
                            _ => (),
                        }
                    }
                });
            }
        });
        let voices = (0..polyphony)
            .map(|i| {
                let note_index = Signal::from_fn({
                    let state = Rc::clone(&state);
                    let effectful_signal = effectful_signal.clone();
                    move |ctx| {
                        effectful_signal.sample(ctx);
                        let state = state.borrow();
                        state.voices[i]
                            .as_ref()
                            .map(|v| v.note_index)
                            .unwrap_or(u7::new(0))
                    }
                });
                let velocity = Signal::from_fn({
                    let state = Rc::clone(&state);
                    let effectful_signal = effectful_signal.clone();
                    move |ctx| {
                        effectful_signal.sample(ctx);
                        let state = state.borrow();
                        state.voices[i]
                            .as_ref()
                            .map(|v| v.velocity)
                            .unwrap_or(u7::new(0))
                    }
                });
                let gate = Gate::from_fn({
                    let state = Rc::clone(&state);
                    let effectful_signal = effectful_signal.clone();
                    move |ctx| {
                        effectful_signal.sample(ctx);
                        let state = state.borrow();
                        state.voices[i].as_ref().map(|v| v.gate).unwrap_or(false)
                    }
                });
                MidiVoiceRaw {
                    note_index,
                    velocity,
                    gate,
                }
            })
            .collect::<Vec<_>>();
        let controllers = MidiControllerTableRaw {
            values: (0..128)
                .map(|i| {
                    Signal::from_fn({
                        let state = Rc::clone(&state);
                        let effectful_signal = effectful_signal.clone();
                        move |ctx| {
                            effectful_signal.sample(ctx);
                            let state = state.borrow();
                            state.controllers[i]
                        }
                    })
                })
                .collect(),
        };
        let pitch_bend = Signal::from_fn({
            let state = Rc::clone(&state);
            let effectful_signal = effectful_signal.clone();
            move |ctx| {
                effectful_signal.sample(ctx);
                let state = state.borrow();
                state.pitch_bend
            }
        });
        MidiPlayerRaw {
            voices,
            controllers,
            pitch_bend,
        }
    }

    pub fn to_player(&self) -> MidiPlayer {
        let pitch_bend_1 = self
            .pitch_bend
            .map(|x| ((x.as_int() as i32 - 0x2000) as f64 * 2.0) / 0x3FFF as f64);
        let pitch_bend_multiplier = pitch_bend_1.map({
            let max_ratio = music::semitone_ratio(2.0);
            move |x| max_ratio.powf(x)
        });
        MidiPlayer {
            voices: self.voices.iter().map(|v| v.to_voice()).collect(),
            controllers: self.controllers.to_controller_table(),
            pitch_bend_1,
            pitch_bend_multiplier,
        }
    }
}
