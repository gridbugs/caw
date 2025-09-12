use caw_core::{Buf, Sig, SigCtx, SigShared, SigT, sig_shared};
use caw_keyboard::{KeyEvent, KeyEvents, Note, TONE_RATIO};
use midly::{MidiMessage, num::u7};
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

/// A collection of simultaneous midi events. When dealing with streams of midi events it's
/// necessary to group them into a collection because multiple midi events may occur during the
/// same frame. This collection only uses the heap when more than one event occurred on the same
/// sample which is very unlikely.
#[derive(Clone, Debug, Default)]
pub struct MidiMessages(SmallVec<[MidiMessage; 1]>);

impl MidiMessages {
    pub fn empty() -> Self {
        Self(smallvec![])
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

    pub fn get_01(&self, index: u8) -> Sig<MidiController01<M>> {
        self.get_with_initial_value_01(index, 0.0)
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
