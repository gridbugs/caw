use crate::{
    music,
    signal::{Freq, Gate, Sf64, Sfreq, Signal, SignalCtx, Su8},
};
pub use midly::{
    num::{u14, u4, u7},
    MidiMessage,
};
use midly::{MetaMessage, PitchBend, Timing, TrackEvent, TrackEventKind};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone, Copy)]
pub struct MidiEvent {
    pub channel: u4,
    pub message: MidiMessage,
}

#[derive(Debug, Clone, Default)]
pub struct MidiEvents {
    events: Vec<MidiEvent>,
}

impl MidiEvents {
    fn add(&mut self, midi_event: MidiEvent) {
        self.events.push(midi_event);
    }

    pub fn for_each_event<F: FnMut(MidiEvent)>(&self, f: F) {
        self.events.iter().cloned().for_each(f)
    }

    pub fn for_each_message_on_channel<F: FnMut(MidiMessage)>(&self, channel: u8, mut f: F) {
        self.for_each_event(|event| {
            if event.channel.as_int() == channel {
                f(event.message);
            }
        })
    }

    pub fn filter_channel(&self, channel: u8) -> MidiMessages {
        let mut midi_messages = MidiMessages::default();
        self.for_each_message_on_channel(channel, |message| midi_messages.add(message));
        midi_messages
    }
}

#[derive(Debug, Clone, Default)]
pub struct MidiMessages {
    messages: Vec<MidiMessage>,
}

impl MidiMessages {
    fn add(&mut self, midi_message: MidiMessage) {
        self.messages.push(midi_message);
    }

    pub fn for_each<F: FnMut(MidiMessage)>(&self, f: F) {
        self.messages.iter().cloned().for_each(f)
    }
}

pub struct MidiVoiceRaw {
    pub note_index: Signal<u7>,
    pub velocity: Signal<u7>,
    pub gate: Gate,
}

#[derive(Clone)]
pub struct MidiVoice {
    pub note_index: Signal<u8>,
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
            note_index: self.note_index.map(|i| i.as_int()),
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

#[derive(Clone)]
pub struct MidiControllerTable {
    values_01: Vec<Sf64>,
}

impl MidiControllerTableRaw {
    pub fn get(&self, i: u8) -> Signal<u7> {
        self.values[i as usize].clone()
    }

    pub fn to_controller_table(&self) -> MidiControllerTable {
        MidiControllerTable {
            values_01: self.values.iter().map(signal_u7_to_01).collect(),
        }
    }
}

impl MidiControllerTable {
    pub fn get_01(&self, i: u8) -> Sf64 {
        self.values_01[i as usize].clone()
    }

    pub fn volume(&self) -> Sf64 {
        self.get_01(7)
    }

    pub fn modulation(&self) -> Sf64 {
        self.get_01(1)
    }
}

const NUM_MIDI_CHANNELS: usize = 16;

pub struct MidiChannelRaw {
    pub voices: Vec<MidiVoiceRaw>,
    pub controllers: MidiControllerTableRaw,
    pub pitch_bend: Signal<u14>,
}

pub struct MidiPlayerRaw {
    pub channels: Vec<MidiChannelRaw>,
}

#[derive(Clone)]
pub struct MidiChannel {
    pub voices: Vec<MidiVoice>,
    pub controllers: MidiControllerTable,
    pub pitch_bend_1: Sf64,
    pub pitch_bend_multiplier_hz: Sf64,
}

pub struct MidiPlayer {
    pub channels: Vec<MidiChannel>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GateState {
    Pressed,
    Released,
    ReleasedThenPressed,
}

struct MidiPlayerRawStateVoice {
    note_index: u7,
    velocity: u7,
    gate: GateState,
    last_change_tick: u64,
}

struct MidiPlayerRawState {
    voices: Vec<MidiPlayerRawStateVoice>,
    controllers: Vec<u7>,
    pitch_bend: u14,
    current_tick: u64,
}

impl MidiPlayerRawState {
    fn allocate_voice(&self) -> usize {
        // try to allocate the the voice whose gate was opened first (of voices whose gates are
        // currently open)
        {
            let mut latest_tick = self.current_tick;
            let mut best_index = None;
            for (i, voice) in self.voices.iter().enumerate() {
                if voice.gate == GateState::Released && voice.last_change_tick < latest_tick {
                    latest_tick = voice.last_change_tick;
                    best_index = Some(i);
                }
            }
            if let Some(i) = best_index {
                return i;
            }
        }
        // all gates are currently closed - allocate the most recently pressed voice
        let mut best_index = 0;
        let mut oldest_tick = 0;
        for (i, voice) in self.voices.iter().enumerate() {
            if voice.last_change_tick > oldest_tick {
                oldest_tick = voice.last_change_tick;
                best_index = i;
            }
        }
        best_index
    }

    fn note_on(&mut self, note_index: u7, velocity: u7) {
        if velocity == 0 {
            self.note_off(note_index);
            return;
        }
        let mut updated = false;
        for voice in self.voices.iter_mut() {
            if voice.note_index == note_index {
                voice.velocity = velocity;
                voice.gate = if voice.last_change_tick == self.current_tick
                    && voice.gate == GateState::Released
                {
                    GateState::ReleasedThenPressed
                } else {
                    GateState::Pressed
                };
                voice.last_change_tick = self.current_tick;
                updated = true;
            }
        }
        if updated {
            return;
        }
        let voice_index = self.allocate_voice();
        let voice = MidiPlayerRawStateVoice {
            note_index,
            velocity,
            gate: GateState::ReleasedThenPressed,
            last_change_tick: self.current_tick,
        };
        self.voices[voice_index] = voice;
    }

    fn note_off(&mut self, note_index: u7) {
        for voice in self.voices.iter_mut() {
            if voice.note_index == note_index {
                voice.gate = GateState::Released;
                voice.last_change_tick = self.current_tick;
            }
        }
    }

    fn progress_gates(&mut self) {
        for voice in self.voices.iter_mut() {
            if voice.gate == GateState::ReleasedThenPressed {
                voice.gate = GateState::Pressed;
            }
        }
    }
}

/// Implement this for types that can produce MIDI events to use them with `MidiPlayer` and
/// `MidiPlayerRaw`.
pub trait MidiEventSource {
    fn for_each_new_event<F>(&mut self, ctx: &SignalCtx, f: F)
    where
        F: FnMut(MidiEvent);
}

pub struct TrackEventSource {
    track: Vec<TrackEvent<'static>>,
    timing: Timing,
    tick_duration_s: f64,
    next_index: usize,
    next_tick: u32,
    current_time_s: f64,
    next_tick_time_s: f64,
}

impl TrackEventSource {
    pub fn new<'a>(track: &[TrackEvent<'a>], timing: Timing, default_s_per_beat: f64) -> Self {
        let track = track
            .into_iter()
            .map(|event| event.to_static())
            .collect::<Vec<_>>();
        let tick_duration_s = match timing {
            Timing::Metrical(ticks_per_beat) => {
                default_s_per_beat / (ticks_per_beat.as_int() as f64)
            }
            Timing::Timecode(frames_per_second, ticks_per_frame) => {
                1.0 / (frames_per_second.as_f32() as f64 * ticks_per_frame as f64)
            }
        };
        Self {
            track,
            timing,
            tick_duration_s,
            next_index: 0,
            next_tick: 0,
            current_time_s: 0.0,
            next_tick_time_s: 0.0,
        }
    }
}

impl MidiEventSource for TrackEventSource {
    fn for_each_new_event<F>(&mut self, ctx: &SignalCtx, mut f: F)
    where
        F: FnMut(MidiEvent),
    {
        let time_since_prev_tick_s = 1.0 / ctx.sample_rate_hz;
        self.current_time_s += time_since_prev_tick_s;
        while self.next_tick_time_s < self.current_time_s {
            self.next_tick_time_s += self.tick_duration_s;
            while let Some(event) = self.track.get(self.next_index) {
                let tick = self.next_tick;
                if event.delta == tick {
                    self.next_tick = 0;
                    self.next_index += 1;
                    match event.kind {
                        TrackEventKind::Midi { channel, message } => {
                            f(MidiEvent { channel, message })
                        }
                        TrackEventKind::Meta(MetaMessage::Tempo(us_per_beat)) => {
                            if let Timing::Metrical(ticks_per_beat) = self.timing {
                                self.tick_duration_s = us_per_beat.as_int() as f64
                                    / (ticks_per_beat.as_int() as f64 * 1_000_000.0);
                            }
                        }
                        _ => (),
                    }
                } else {
                    break;
                }
            }
            self.next_tick += 1;
        }
    }
}

impl MidiPlayerRaw {
    pub fn new(polyphony: usize, event_source: impl MidiEventSource + 'static) -> Self {
        let event_source = RefCell::new(event_source);
        let states = Rc::new(RefCell::new(
            (0..NUM_MIDI_CHANNELS)
                .map(|_| MidiPlayerRawState {
                    voices: (0..polyphony)
                        .map(|_| MidiPlayerRawStateVoice {
                            note_index: 0.into(),
                            velocity: 0.into(),
                            gate: GateState::Released,
                            last_change_tick: 0,
                        })
                        .collect(),
                    controllers: (0..128).map(|_| u7::new(0)).collect(),
                    pitch_bend: u14::new(0x2000),
                    current_tick: 0,
                })
                .collect::<Vec<_>>(),
        ));
        let effectful_signal = Signal::from_fn({
            let states = Rc::clone(&states);
            move |ctx| {
                let mut states = states.borrow_mut();
                for state in states.iter_mut() {
                    state.progress_gates();
                }
                event_source.borrow_mut().for_each_new_event(ctx, |event| {
                    let state = &mut states[event.channel.as_int() as usize];
                    use MidiMessage::*;
                    match event.message {
                        NoteOn { key, vel } => state.note_on(key, vel),
                        NoteOff { key, vel: _ } => state.note_off(key),
                        PitchBend {
                            bend: PitchBend(pitch_bend),
                        } => state.pitch_bend = pitch_bend,
                        Controller { controller, value } => {
                            state.controllers[controller.as_int() as usize] = value;
                        }
                        _ => (),
                    }
                });
                for state in states.iter_mut() {
                    state.current_tick += 1;
                }
            }
        });
        let channels = (0..NUM_MIDI_CHANNELS)
            .map(|channel_i| {
                let voices = (0..polyphony)
                    .map(|i| {
                        let note_index = Signal::from_fn({
                            let states = Rc::clone(&states);
                            let effectful_signal = effectful_signal.clone();
                            move |ctx| {
                                effectful_signal.sample(ctx);
                                states.borrow()[channel_i].voices[i].note_index
                            }
                        });
                        let velocity = Signal::from_fn({
                            let states = Rc::clone(&states);
                            let effectful_signal = effectful_signal.clone();
                            move |ctx| {
                                effectful_signal.sample(ctx);
                                states.borrow()[channel_i].voices[i].velocity
                            }
                        });
                        let gate = Gate::from_fn({
                            let states = Rc::clone(&states);
                            let effectful_signal = effectful_signal.clone();
                            move |ctx| {
                                effectful_signal.sample(ctx);
                                let gate_state = states.borrow()[channel_i].voices[i].gate;
                                gate_state == GateState::Pressed
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
                                let states = Rc::clone(&states);
                                let effectful_signal = effectful_signal.clone();
                                move |ctx| {
                                    effectful_signal.sample(ctx);
                                    states.borrow()[channel_i].controllers[i]
                                }
                            })
                        })
                        .collect(),
                };
                let pitch_bend = Signal::from_fn({
                    let states = Rc::clone(&states);
                    let effectful_signal = effectful_signal.clone();
                    move |ctx| {
                        effectful_signal.sample(ctx);
                        states.borrow()[channel_i].pitch_bend
                    }
                });
                MidiChannelRaw {
                    voices,
                    controllers,
                    pitch_bend,
                }
            })
            .collect();
        Self { channels }
    }

    pub fn to_player(&self) -> MidiPlayer {
        let channels = self
            .channels
            .iter()
            .map(|channel_raw| channel_raw.to_channel())
            .collect();
        MidiPlayer { channels }
    }
}

impl MidiChannelRaw {
    fn to_channel(&self) -> MidiChannel {
        let pitch_bend_1 = self
            .pitch_bend
            .map(|x| ((x.as_int() as i32 - 0x2000) as f64 * 2.0) / 0x3FFF as f64);
        let pitch_bend_multiplier_hz = pitch_bend_1.map({
            let max_ratio = music::semitone_ratio(2.0);
            move |x| max_ratio.powf(x)
        });
        MidiChannel {
            voices: self.voices.iter().map(|v| v.to_voice()).collect(),
            controllers: self.controllers.to_controller_table(),
            pitch_bend_1,
            pitch_bend_multiplier_hz,
        }
    }
}

impl MidiPlayer {
    pub fn new(polyphony: usize, event_source: impl MidiEventSource + 'static) -> Self {
        MidiPlayerRaw::new(polyphony, event_source).to_player()
    }
}

pub struct MidiPlayerMonophonicRaw {
    pub voice: MidiVoiceRaw,
    pub controllers: MidiControllerTableRaw,
    pub pitch_bend: Signal<u14>,
}

pub struct MidiPlayerMonophonic {
    pub voice: MidiVoice,
    pub controllers: MidiControllerTable,
    pub pitch_bend_1: Sf64,
    pub pitch_bend_multiplier_hz: Sf64,
}

struct MidiPlayerMonophonicRawStateNote {
    velocity: u7,
    last_pressed: u64,
    currently_pressed: bool,
}

struct MidiPlayerMonophonicRawState {
    notes: Vec<MidiPlayerMonophonicRawStateNote>,
    controllers: Vec<u7>,
    pitch_bend: u14,
}

#[derive(Clone, Copy, Default)]
struct MidiPlayerMonophonicRawVoiceComponents {
    note_index: u7,
    velocity: u7,
    gate: bool,
}

impl MidiPlayerMonophonicRawState {
    fn voice_components(&self) -> MidiPlayerMonophonicRawVoiceComponents {
        let mut any_currently_pressed = false;
        let mut note_index = u7::new(0);
        let mut newest_last_pressed = 0;
        let mut velocity = u7::new(0);
        let mut note_index_all = u7::new(0);
        let mut newest_last_pressed_all = 0;
        for (i, note) in self.notes.iter().enumerate() {
            if note.currently_pressed {
                any_currently_pressed = true;
                if note.last_pressed > newest_last_pressed {
                    newest_last_pressed = note.last_pressed;
                    note_index = (i as u8).into();
                    velocity = note.velocity;
                }
            }
            if note.last_pressed > newest_last_pressed_all {
                newest_last_pressed_all = note.last_pressed;
                note_index_all = (i as u8).into();
            }
        }
        let note_index = if any_currently_pressed {
            note_index
        } else {
            note_index_all
        };
        MidiPlayerMonophonicRawVoiceComponents {
            note_index,
            velocity,
            gate: any_currently_pressed,
        }
    }
}

impl MidiPlayerMonophonicRawState {
    fn new() -> Self {
        Self {
            notes: (0..128)
                .map(|_| MidiPlayerMonophonicRawStateNote {
                    velocity: 0.into(),
                    last_pressed: 0,
                    currently_pressed: false,
                })
                .collect(),
            controllers: (0..128).map(|_| u7::new(0)).collect(),
            pitch_bend: u14::new(0x2000),
        }
    }
}

impl MidiPlayerMonophonicRaw {
    pub fn new(channel: u4, event_source: impl MidiEventSource + 'static) -> Self {
        let state = Rc::new(RefCell::new(MidiPlayerMonophonicRawState::new()));
        let event_source = RefCell::new(event_source);
        let effectful_signal = Signal::from_fn({
            let state = Rc::clone(&state);
            move |ctx| {
                event_source.borrow_mut().for_each_new_event(ctx, |event| {
                    if event.channel == channel {
                        let mut state = state.borrow_mut();
                        use MidiMessage::*;
                        match event.message {
                            NoteOn { key, vel: velocity } => {
                                state.notes[key.as_int() as usize] =
                                    MidiPlayerMonophonicRawStateNote {
                                        last_pressed: ctx.sample_index,
                                        currently_pressed: true,
                                        velocity,
                                    }
                            }
                            NoteOff { key, vel: _ } => {
                                state.notes[key.as_int() as usize].currently_pressed = false;
                            }
                            PitchBend {
                                bend: PitchBend(pitch_bend),
                            } => state.pitch_bend = pitch_bend,
                            Controller { controller, value } => {
                                state.controllers[controller.as_int() as usize] = value;
                            }
                            _ => (),
                        }
                    }
                })
            }
        });
        let voice_components = Signal::from_fn({
            let state = Rc::clone(&state);
            let effectful_signal = effectful_signal.clone();
            move |ctx| {
                effectful_signal.sample(ctx);
                state.borrow().voice_components()
            }
        });
        let voice = MidiVoiceRaw {
            note_index: voice_components.map(|v| v.note_index),
            velocity: voice_components.map(|v| v.velocity),
            gate: voice_components.map(|v| v.gate).to_gate(),
        };
        let controllers = MidiControllerTableRaw {
            values: (0..128)
                .map(|i| {
                    Signal::from_fn({
                        let state = Rc::clone(&state);
                        let effectful_signal = effectful_signal.clone();
                        move |ctx| {
                            effectful_signal.sample(ctx);
                            state.borrow().controllers[i]
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
                state.borrow().pitch_bend
            }
        });
        Self {
            voice,
            controllers,
            pitch_bend,
        }
    }

    pub fn to_player(&self) -> MidiPlayerMonophonic {
        let pitch_bend_1 = self
            .pitch_bend
            .map(|x| ((x.as_int() as i32 - 0x2000) as f64 * 2.0) / 0x3FFF as f64);
        let pitch_bend_multiplier_hz = pitch_bend_1.map({
            let max_ratio = music::semitone_ratio(2.0);
            move |x| max_ratio.powf(x)
        });
        MidiPlayerMonophonic {
            voice: self.voice.to_voice(),
            controllers: self.controllers.to_controller_table(),
            pitch_bend_1,
            pitch_bend_multiplier_hz,
        }
    }
}

impl MidiPlayerMonophonic {
    pub fn new(channel: u4, event_source: impl MidiEventSource + 'static) -> Self {
        MidiPlayerMonophonicRaw::new(channel, event_source).to_player()
    }
}

pub fn event_source_to_signal(event_source: impl MidiEventSource + 'static) -> Signal<MidiEvents> {
    let event_source = RefCell::new(event_source);
    Signal::from_fn(move |ctx| {
        let mut midi_events = MidiEvents::default();
        event_source
            .borrow_mut()
            .for_each_new_event(ctx, |midi_event| midi_events.add(midi_event));
        midi_events
    })
}

impl Signal<MidiEvents> {
    pub fn filter_channel(&self, channel: u8) -> Signal<MidiMessages> {
        self.map(move |events| events.filter_channel(channel))
    }
}

pub fn event_source_to_signal_single_channel(
    event_source: impl MidiEventSource + 'static,
    channel: u8,
) -> Signal<MidiMessages> {
    let event_source = RefCell::new(event_source);
    Signal::from_fn(move |ctx| {
        let mut midi_messages = MidiMessages::default();
        event_source
            .borrow_mut()
            .for_each_new_event(ctx, |midi_event| {
                if midi_event.channel == channel {
                    midi_messages.add(midi_event.message);
                }
            });
        midi_messages
    })
}

pub struct MidiMonophonic {
    pub note_index: Su8,
    pub velocity_01: Sf64,
    pub gate: Gate,
}

impl Signal<MidiMessages> {
    pub fn controller_table(&self) -> MidiControllerTable {
        let table = Rc::new(RefCell::new(vec![0.0; 128]));
        let update_table = Signal::from_fn({
            let table = Rc::clone(&table);
            let signal = self.clone();
            move |ctx| {
                let mut table = table.borrow_mut();
                signal.sample(ctx).for_each(|message| match message {
                    MidiMessage::Controller { controller, value } => {
                        table[controller.as_int() as usize] = value.as_int() as f64 / 127.0;
                    }
                    _ => (),
                });
            }
        });
        MidiControllerTable {
            values_01: (0..128)
                .map(|i| {
                    update_table.map({
                        let table = Rc::clone(&table);
                        move |()| {
                            let x = table.borrow()[i];
                            x
                        }
                    })
                })
                .collect(),
        }
    }

    pub fn monophonic(&self) -> MidiMonophonic {
        struct State {
            note_index: u8,
            velocity_01: f64,
            key_down: bool,
        }
        let state = Rc::new(RefCell::new(State {
            note_index: 0,
            velocity_01: 0.0,
            key_down: false,
        }));
        let update_state = self.map({
            let state = Rc::clone(&state);
            move |messages| {
                let mut state = state.borrow_mut();
                messages.for_each(|message| match message {
                    MidiMessage::NoteOn { key, vel } => {
                        state.velocity_01 = vel.as_int() as f64 / 127.0;
                        state.note_index = key.as_int();
                        state.key_down = true;
                    }
                    MidiMessage::NoteOff { key, vel } => {
                        state.velocity_01 = vel.as_int() as f64 / 127.0;
                        state.note_index = key.as_int();
                        state.key_down = false;
                    }
                    _ => (),
                });
            }
        });
        let (note_index, velocity_01, gate) = update_state
            .map({
                let state = Rc::clone(&state);
                move |()| {
                    let state = state.borrow();
                    (state.note_index, state.velocity_01, state.key_down)
                }
            })
            .unzip();
        MidiMonophonic {
            note_index,
            velocity_01,
            gate: gate.to_gate(),
        }
    }
}
