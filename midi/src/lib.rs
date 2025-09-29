use caw_core::{Buf, Sig, SigCtx, SigShared, SigT, sig_shared};
use caw_keyboard::{KeyEvent, KeyEvents, Note, TONE_RATIO};
use midly::{
    MidiMessage,
    num::{u4, u7},
};
use smallvec::{SmallVec, smallvec};

fn u7_to_01(u7: u7) -> f32 {
    u7.as_int() as f32 / 127.0
}

fn midi_note_message_to_key_event(key: u7, vel: u7, pressed: bool) -> KeyEvent {
    KeyEvent {
        note: Note::from_midi_index(key),
        velocity_01: u7_to_01(vel),
        pressed,
    }
}

fn midi_message_to_key_event(midi_message: MidiMessage) -> Option<KeyEvent> {
    match midi_message {
        MidiMessage::NoteOn { key, vel } => {
            Some(midi_note_message_to_key_event(key, vel, true))
        }
        MidiMessage::NoteOff { key, vel } => {
            Some(midi_note_message_to_key_event(key, vel, false))
        }
        _ => None,
    }
}

/// A midi message on a channel.
#[derive(Clone, Debug)]
pub struct MidiEvent {
    pub channel: u4,
    pub message: MidiMessage,
}

/// A collection of simultaneous midi events. When dealing with streams of midi events it's
/// necessary to group them into a collection because multiple midi events may occur during the
/// same frame. This collection only uses the heap when more than one event occurred on the same
/// sample which is very unlikely.
#[derive(Clone, Debug, Default)]
pub struct MidiEvents(SmallVec<[MidiEvent; 1]>);

impl MidiEvents {
    pub fn empty() -> Self {
        Self(smallvec![])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push(&mut self, midi_event: MidiEvent) {
        self.0.push(midi_event);
    }

    pub fn iter(&self) -> impl Iterator<Item = &MidiEvent> {
        self.0.iter()
    }
}

impl IntoIterator for MidiEvents {
    type Item = MidiEvent;

    type IntoIter = smallvec::IntoIter<[MidiEvent; 1]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A collection of simultaneous midi messages. When dealing with streams of midi messages it's
/// necessary to group them into a collection because multiple midi messages may arrive during the
/// same frame. This collection only uses the heap when more than one event occurred on the same
/// sample which is very unlikely.
#[derive(Clone, Debug, Default)]
pub struct MidiMessages(SmallVec<[MidiMessage; 1]>);

impl MidiMessages {
    pub fn empty() -> Self {
        Self(smallvec![])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn push(&mut self, midi_event: MidiMessage) {
        self.0.push(midi_event);
    }

    pub fn iter(&self) -> impl Iterator<Item = &MidiMessage> {
        self.0.iter()
    }

    pub fn key_events(&self) -> KeyEvents {
        self.iter()
            .cloned()
            .filter_map(midi_message_to_key_event)
            .collect()
    }
}

impl IntoIterator for MidiMessages {
    type Item = MidiMessage;

    type IntoIter = smallvec::IntoIter<[MidiMessage; 1]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Clone)]
pub struct MidiControllers<M>
where
    M: SigT<Item = MidiMessages>,
{
    messages: Sig<SigShared<M>>,
}

pub struct MidiController01<M>
where
    M: SigT<Item = MidiMessages>,
{
    index: u7,
    state: f32,
    messages: SigShared<M>,
    buf: Vec<f32>,
}

impl<M> SigT for MidiController01<M>
where
    M: SigT<Item = MidiMessages>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        let messages = self.messages.sample(ctx);
        for (out, messages) in self.buf.iter_mut().zip(messages.iter()) {
            for message in messages {
                if let MidiMessage::Controller { controller, value } = message {
                    if controller == self.index {
                        self.state = value.as_int() as f32 / 127.0;
                    }
                }
            }
            *out = self.state;
        }
        &self.buf
    }
}

pub struct MidiControllerU7<M>
where
    M: SigT<Item = MidiMessages>,
{
    index: u7,
    state: u8,
    messages: SigShared<M>,
    buf: Vec<u8>,
}

impl<M> SigT for MidiControllerU7<M>
where
    M: SigT<Item = MidiMessages>,
{
    type Item = u8;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0);
        let messages = self.messages.sample(ctx);
        for (out, messages) in self.buf.iter_mut().zip(messages.iter()) {
            for message in messages {
                if let MidiMessage::Controller { controller, value } = message {
                    if controller == self.index {
                        self.state = value.as_int();
                    }
                }
            }
            *out = self.state;
        }
        &self.buf
    }
}

/// A bool which is true if the controller has a non-zero value
pub struct MidiControllerBool<M>
where
    M: SigT<Item = MidiMessages>,
{
    index: u7,
    state: bool,
    messages: SigShared<M>,
    buf: Vec<bool>,
}

impl<M> SigT for MidiControllerBool<M>
where
    M: SigT<Item = MidiMessages>,
{
    type Item = bool;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, false);
        let messages = self.messages.sample(ctx);
        for (out, messages) in self.buf.iter_mut().zip(messages.iter()) {
            for message in messages {
                if let MidiMessage::Controller { controller, value } = message {
                    if controller == self.index {
                        self.state = value.as_int() > 0;
                    }
                }
            }
            *out = self.state;
        }
        &self.buf
    }
}

/// A signal representing the state of a single midi key
pub struct MidiKeyPress<M>
where
    M: SigT<Item = MidiMessages>,
{
    messages: SigShared<M>,
    state: bool,
    note: Note,
    buf: Vec<bool>,
}

impl<M> SigT for MidiKeyPress<M>
where
    M: SigT<Item = MidiMessages>,
{
    type Item = bool;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, false);
        let messages = self.messages.sample(ctx);
        for (out, messages) in self.buf.iter_mut().zip(messages.iter()) {
            for message in messages {
                match message {
                    MidiMessage::NoteOn { key, .. } => {
                        if key.as_int() == self.note.to_midi_index() {
                            self.state = true;
                        }
                    }
                    MidiMessage::NoteOff { key, .. } => {
                        if key.as_int() == self.note.to_midi_index() {
                            self.state = false;
                        }
                    }
                    _ => (),
                }
            }
            *out = self.state;
        }
        &self.buf
    }
}

impl<M> MidiControllers<M>
where
    M: SigT<Item = MidiMessages>,
{
    pub fn get_with_initial_value_01(
        &self,
        index: u8,
        initial_value: f32,
    ) -> Sig<MidiController01<M>> {
        Sig(MidiController01 {
            index: index.into(),
            state: initial_value.clamp(0., 1.),
            messages: self.messages.clone().0,
            buf: Vec::new(),
        })
    }

    pub fn get_with_initial_value_u7(
        &self,
        index: u8,
        initial_value: u8,
    ) -> Sig<MidiControllerU7<M>> {
        Sig(MidiControllerU7 {
            index: index.into(),
            state: initial_value.into(),
            messages: self.messages.clone().0,
            buf: Vec::new(),
        })
    }

    pub fn get_with_initial_value_bool(
        &self,
        index: u8,
        initial_value: bool,
    ) -> Sig<MidiControllerBool<M>> {
        Sig(MidiControllerBool {
            index: index.into(),
            state: initial_value,
            messages: self.messages.clone().0,
            buf: Vec::new(),
        })
    }

    pub fn get_01(&self, index: u8) -> Sig<MidiController01<M>> {
        self.get_with_initial_value_01(index, 0.0)
    }

    pub fn get_u7(&self, index: u8) -> Sig<MidiControllerU7<M>> {
        self.get_with_initial_value_u7(index, 0)
    }

    pub fn get_bool(&self, index: u8) -> Sig<MidiControllerBool<M>> {
        self.get_with_initial_value_bool(index, false)
    }

    pub fn volume(&self) -> Sig<MidiController01<M>> {
        self.get_01(7)
    }

    pub fn modulation(&self) -> Sig<MidiController01<M>> {
        self.get_01(1)
    }
}

pub trait MidiMessagesT<M>
where
    M: SigT<Item = MidiMessages>,
{
    fn key_events(self) -> Sig<impl SigT<Item = KeyEvents>>;

    /// The pitch bend value interpolated between -1 and 1
    fn pitch_bend_raw(self) -> Sig<impl SigT<Item = f32>>;

    /// The pitch bend value as a value that can be multiplied by a frequence to scale it up or
    /// down by at most one tone.
    fn pitch_bend_freq_mult(self) -> Sig<impl SigT<Item = f32>>;

    fn controllers(self) -> MidiControllers<M>;

    /// Return a signal that reports the state of a given key over time.
    fn key_press(self, note: Note) -> MidiKeyPress<M>;
}

impl<M> MidiMessagesT<M> for Sig<M>
where
    M: SigT<Item = MidiMessages>,
{
    fn key_events(self) -> Sig<impl SigT<Item = KeyEvents>> {
        self.map(|midi_messages| midi_messages.key_events())
    }

    fn pitch_bend_raw(self) -> Sig<impl SigT<Item = f32>> {
        let mut state = 0.0;
        self.map_mut(move |midi_messages| {
            for midi_message in midi_messages {
                if let MidiMessage::PitchBend { bend } = midi_message {
                    state = bend.as_f32();
                }
            }
            state
        })
    }

    fn pitch_bend_freq_mult(self) -> Sig<impl SigT<Item = f32>> {
        self.pitch_bend_raw().map(|bend| TONE_RATIO.powf(bend))
    }

    fn controllers(self) -> MidiControllers<M> {
        MidiControllers {
            messages: sig_shared(self.0),
        }
    }

    fn key_press(self, note: Note) -> MidiKeyPress<M> {
        MidiKeyPress {
            messages: sig_shared(self.0).0,
            state: false,
            note,
            buf: Vec::new(),
        }
    }
}

pub struct MidiChannel<E>
where
    E: SigT<Item = MidiEvents>,
{
    midi_events: E,
    channel: u4,
    buf: Vec<MidiMessages>,
}

impl<E> SigT for MidiChannel<E>
where
    E: SigT<Item = MidiEvents>,
{
    type Item = MidiMessages;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize_with(ctx.num_samples, Default::default);
        let midi_events = self.midi_events.sample(ctx);
        {
            // Only copy messages from the first sample worth of events.
            let midi_messages = &mut self.buf[0];
            midi_messages.clear();
            if let Some(midi_events) = midi_events.iter().next() {
                for midi_event in midi_events {
                    if midi_event.channel == self.channel {
                        midi_messages.push(midi_event.message);
                    }
                }
            }
        }
        &self.buf
    }
}

pub trait MidiEventsT<E>
where
    E: SigT<Item = MidiEvents>,
{
    fn channel(self, channel: u8) -> Sig<MidiChannel<E>>;
}

impl<E> MidiEventsT<E> for Sig<E>
where
    E: SigT<Item = MidiEvents>,
{
    fn channel(self, channel: u8) -> Sig<MidiChannel<E>> {
        if let Some(channel) = u4::try_from(channel) {
            Sig(MidiChannel {
                midi_events: self.0,
                channel,
                buf: Vec::new(),
            })
        } else {
            panic!("Invalid midi channel: {}", channel)
        }
    }
}
