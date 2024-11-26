use crate::{
    keyboard::KeyEvent,
    music::{self, Note},
    signal::{Sf64, Signal, SignalCtx},
};
pub use midly::{
    num::{u14, u4, u7},
    MidiMessage,
};
use midly::{MetaMessage, PitchBend, Timing, TrackEvent, TrackEventKind};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

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

    pub fn for_each_message_on_channel<F: FnMut(MidiMessage)>(
        &self,
        channel: u8,
        mut f: F,
    ) {
        self.for_each_event(|event| {
            if event.channel.as_int() == channel {
                f(event.message);
            }
        })
    }

    pub fn filter_channel(&self, channel: u8) -> MidiMessages {
        let mut midi_messages = MidiMessages::default();
        self.for_each_message_on_channel(channel, |message| {
            midi_messages.add(message)
        });
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

fn u7_to_01(u7: u7) -> f64 {
    u7.as_int() as f64 / 127.0
}

fn signal_u7_to_01(s: &Signal<u7>) -> Sf64 {
    s.map(u7_to_01)
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

/// A type that can produce a stream of midi events such as a midi file or device
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
    pub fn new(
        track: &[TrackEvent<'_>],
        timing: Timing,
        default_s_per_beat: f64,
    ) -> Self {
        let track = track
            .iter()
            .map(|event| event.to_static())
            .collect::<Vec<_>>();
        let tick_duration_s = match timing {
            Timing::Metrical(ticks_per_beat) => {
                default_s_per_beat / (ticks_per_beat.as_int() as f64)
            }
            Timing::Timecode(frames_per_second, ticks_per_frame) => {
                1.0 / (frames_per_second.as_f32() as f64
                    * ticks_per_frame as f64)
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
                        TrackEventKind::Meta(MetaMessage::Tempo(
                            us_per_beat,
                        )) => {
                            if let Timing::Metrical(ticks_per_beat) =
                                self.timing
                            {
                                self.tick_duration_s = us_per_beat.as_int()
                                    as f64
                                    / (ticks_per_beat.as_int() as f64
                                        * 1_000_000.0);
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

pub fn event_source_to_signal(
    event_source: impl MidiEventSource + 'static,
) -> Signal<MidiEvents> {
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

fn midi_note_message_to_key_event(key: u7, vel: u7, pressed: bool) -> KeyEvent {
    KeyEvent {
        note: Note::from_midi_index(key),
        velocity_01: u7_to_01(vel),
        pressed,
    }
}

fn midi_pitch_bend_to_pitch_bend_multiplier_hz(midi_pitch_bend: u14) -> f64 {
    music::TONE_RATIO.powf(
        ((midi_pitch_bend.as_int() as i32 - 0x2000) as f64 * 2.0)
            / 0x3FFF as f64,
    )
}

impl Signal<MidiMessages> {
    pub fn key_events(&self) -> Signal<Vec<KeyEvent>> {
        self.map(|messages| {
            let mut ret = Vec::new();
            messages.for_each(|message| match message {
                MidiMessage::NoteOn { key, vel } => {
                    ret.push(midi_note_message_to_key_event(key, vel, true))
                }
                MidiMessage::NoteOff { key, vel } => {
                    ret.push(midi_note_message_to_key_event(key, vel, false))
                }
                _ => (),
            });
            ret
        })
    }

    pub fn pitch_bend_multiplier_hz(&self) -> Sf64 {
        let state = Rc::new(Cell::new(1.0));
        self.map({
            let state = Rc::clone(&state);
            move |messages| {
                messages.for_each(|message| if let MidiMessage::PitchBend {
                        bend: PitchBend(pitch_bend),
                    } = message { state.set(
                    midi_pitch_bend_to_pitch_bend_multiplier_hz(pitch_bend),
                ) });
            }
        })
        .then({
            let state = Rc::clone(&state);
            move || state.get()
        })
    }

    pub fn controller_table(&self) -> MidiControllerTable {
        let table = Rc::new(RefCell::new(vec![0.0; 128]));
        let update_table = Signal::from_fn({
            let table = Rc::clone(&table);
            let signal = self.clone();
            move |ctx| {
                let mut table = table.borrow_mut();
                signal.sample(ctx).for_each(|message| if let MidiMessage::Controller { controller, value } = message {
                    table[controller.as_int() as usize] =
                        value.as_int() as f64 / 127.0;
                });
            }
        });
        MidiControllerTable {
            values_01: (0..128)
                .map(|i| {
                    update_table.map({
                        let table = Rc::clone(&table);
                        move |()| table.borrow()[i]
                    })
                })
                .collect(),
        }
    }
}
