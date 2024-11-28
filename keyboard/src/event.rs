use crate::{polyphony, MonoVoice, Note};
use caw_core_next::{FrameSig, FrameSigT};
use smallvec::{smallvec, SmallVec};

/// A key being pressed or released
#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    /// Which note corresponds to the key
    pub note: Note,
    /// Whether the key was pressed or released
    pub pressed: bool,
    /// How hard the key was pressed/released
    pub velocity_01: f32,
}

/// A collection of simultaneous key events. When dealing with streams of key events it's necessary
/// to group them into a collection because multiple key events may occur during the same audio
/// sample. This collection only uses the heap when more than one event occurred on the same sample
/// which is very unlikely.
#[derive(Clone, Debug)]
pub struct KeyEvents(SmallVec<[KeyEvent; 1]>);

impl KeyEvents {
    pub fn empty() -> Self {
        Self(smallvec![])
    }

    pub fn push(&mut self, key_event: KeyEvent) {
        self.0.push(key_event);
    }

    pub fn last(&self) -> Option<&KeyEvent> {
        self.0.last()
    }

    pub fn iter(&self) -> impl Iterator<Item = &KeyEvent> {
        self.0.iter()
    }
}

impl IntoIterator for KeyEvents {
    type Item = KeyEvent;

    type IntoIter = smallvec::IntoIter<[KeyEvent; 1]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub trait IntoVoice {
    fn mono_voice(self) -> MonoVoice<impl FrameSigT<Item = KeyEvents>>;
    fn poly_voices(
        self,
        n: usize,
    ) -> Vec<MonoVoice<impl FrameSigT<Item = KeyEvents>>>;
}

impl<K> IntoVoice for FrameSig<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    fn mono_voice(self) -> MonoVoice<impl FrameSigT<Item = KeyEvents>> {
        MonoVoice::from_key_events(self.0)
    }

    fn poly_voices(
        self,
        n: usize,
    ) -> Vec<MonoVoice<impl FrameSigT<Item = KeyEvents>>> {
        polyphony::voices_from_key_events(self.0, n)
    }
}
