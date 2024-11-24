use crate::Note;
use caw_core_next::SigT;
use smallvec::{smallvec, SmallVec};
use std::{cell::RefCell, rc::Rc};

/// A key being pressed or released
#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    /// Which note corresponds to the key
    pub note: Note,
    /// Whether the key was pressed or released
    pub pressed: bool,
    /// How hard the key was pressed/released
    pub velocity_01: f64,
}

/// A collection of simultaneous key events. When dealing with streams of key events it's necessary
/// to group them into a collection because multiple key events may occur during the same audio
/// sample. This collection only uses the heap when more than one event occurred on the same sample
/// which is very unlikely.
pub struct KeyEvents(SmallVec<[KeyEvent; 1]>);

impl KeyEvents {
    pub fn empty() -> Self {
        Self(smallvec![])
    }

    pub fn push(&mut self, key_event: KeyEvent) {
        self.0.push(key_event);
    }
}

/// A collection of signals associated with a monophonic keyboard voice
pub struct MonoVoice<N, D, P, V>
where
    N: SigT<Item = Note>,
    D: SigT<Item = bool>,
    P: SigT<Item = bool>,
    V: SigT<Item = f32>,
{
    /// Stream of the current or most recent note played. This always yields a note even if no key
    /// is currently down as the sound of the note may still be playing, such as when an envelope
    /// is not finished releasing. The note yielded before any key has been pressed is arbitrary.
    pub note: N,
    /// True the entire time a key is held
    pub key_down_gate: D,
    /// True on the first audio sample during which a key is held
    pub key_press_trigger: P,
    /// How hard the most recently pressed/released key was pressed/released
    pub velocity_01: V,
}

type X<K: SigT<Item = KeyEvents>> = Rc<RefCell<K>>;

//fn foo(x: Rc<RefCell<dyn SigT<Item = KeyEvents>>>) {}

impl<N, D, P, V> MonoVoice<N, D, P, V>
where
    N: SigT<Item = Note>,
    D: SigT<Item = bool>,
    P: SigT<Item = bool>,
    V: SigT<Item = f32>,
{
    fn from_key_events_sig<K>(key_events_sig: K)
    where
        K: SigT<Item = KeyEvents>,
    {
    }
}
