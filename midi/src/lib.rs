use caw_core_next::{FrameSig, FrameSigT};
use caw_keyboard::{KeyEvent, KeyEvents, Note};
use midly::{num::u7, MidiMessage};
use smallvec::{smallvec, SmallVec};

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

fn midi_messages_to_key_events<M>(
    midi_messages: M,
) -> FrameSig<impl FrameSigT<Item = KeyEvents>>
where
    M: FrameSigT<Item = MidiMessages>,
{
    FrameSig(midi_messages).map(|midi_messages| midi_messages.key_events())
}

pub trait IntoKeyEvents<M>
where
    M: FrameSigT<Item = MidiMessages>,
{
    fn key_events(self) -> FrameSig<impl FrameSigT<Item = KeyEvents>>;
}

impl<M> IntoKeyEvents<M> for FrameSig<M>
where
    M: FrameSigT<Item = MidiMessages>,
{
    fn key_events(self) -> FrameSig<impl FrameSigT<Item = KeyEvents>> {
        midi_messages_to_key_events(self)
    }
}
