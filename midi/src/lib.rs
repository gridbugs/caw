use caw_core_next::{FrameSig, FrameSigT};
use caw_keyboard::{KeyEvent, KeyEvents, Note, TONE_RATIO};
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

pub trait MidiMessagesT<M>
where
    M: FrameSigT<Item = MidiMessages>,
{
    fn key_events(self) -> FrameSig<impl FrameSigT<Item = KeyEvents>>;

    /// The pitch bend value interpolated between -1 and 1
    fn pitch_bend_raw(self) -> FrameSig<impl FrameSigT<Item = f32>>;

    /// The pitch bend value as a value that can be multiplied by a frequence to scale it up or
    /// down by at most one tone.
    fn pitch_bend_freq_mult(self) -> FrameSig<impl FrameSigT<Item = f32>>;
}

impl<M> MidiMessagesT<M> for FrameSig<M>
where
    M: FrameSigT<Item = MidiMessages>,
{
    fn key_events(self) -> FrameSig<impl FrameSigT<Item = KeyEvents>> {
        self.map(|midi_messages| midi_messages.key_events())
    }

    fn pitch_bend_raw(self) -> FrameSig<impl FrameSigT<Item = f32>> {
        let mut state = 0.0;
        self.map(move |midi_messages| {
            for midi_message in midi_messages {
                if let MidiMessage::PitchBend { bend } = midi_message {
                    state = bend.as_f32();
                }
            }
            state
        })
    }

    fn pitch_bend_freq_mult(self) -> FrameSig<impl FrameSigT<Item = f32>> {
        self.pitch_bend_raw().map(|bend| TONE_RATIO.powf(bend))
    }
}
