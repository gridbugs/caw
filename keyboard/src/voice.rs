use crate::Note;
use caw_core_next::{Sig, SigShared, SigT};
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

/// A collection of signals associated with a monophonic keyboard voice
pub struct MonoVoice<K, D, P>
where
    K: SigT<Item = KeyEvents>,
    D: SigT<Item = bool>,
    P: SigT<Item = bool>,
{
    /// Stream of the current or most recent note played. This always yields a note even if no key
    /// is currently down as the sound of the note may still be playing, such as when an envelope
    /// is not finished releasing. The note yielded before any key has been pressed is arbitrary.
    pub note: Sig<mono_voice::Note<K>>,
    /// True the entire time a key is held
    pub key_down_gate: D,
    /// True on the first audio sample during which a key is held
    pub key_press_trigger: P,
    /// How hard the most recently pressed/released key was pressed/released
    pub velocity_01: Sig<mono_voice::Velocity<K>>,
}

pub mod mono_voice {
    use super::KeyEvents;
    use caw_core_next::{Buf, SigCtx, SigShared, SigT};

    /// Extracts a sequence of notes from a sequence of key events.
    pub struct Note<K>
    where
        K: SigT<Item = KeyEvents>,
    {
        sig_shared: SigShared<K>,
        held_notes: Vec<crate::Note>,
        last_note: crate::Note,
        buf: Vec<crate::Note>,
    }

    impl<K> Note<K>
    where
        K: SigT<Item = KeyEvents>,
    {
        pub fn new(sig_shared: SigShared<K>) -> Self {
            Self {
                sig_shared,
                held_notes: Vec::new(),
                last_note: crate::Note::C4, // arbitrarily default to middle C
                buf: Vec::new(),
            }
        }
    }

    impl<K> SigT for Note<K>
    where
        K: SigT<Item = KeyEvents>,
    {
        type Item = crate::Note;

        fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
            self.buf.clear();
            self.sig_shared.for_each_item(ctx, |events| {
                for event in events.iter() {
                    // Remove the held note if it already exists. This assumes there are no duplicate
                    // note in the vector.
                    for (i, &note) in self.held_notes.iter().enumerate() {
                        if note == event.note {
                            self.held_notes.remove(i);
                            break;
                        }
                    }
                    if event.pressed {
                        self.held_notes.push(event.note);
                    }
                    self.last_note = event.note;
                }
                // Yield the most-recently pressed note of all held notes, or the last touched note
                // if no notes are currently held.
                self.buf
                    .push(*self.held_notes.last().unwrap_or(&self.last_note))
            });
            &self.buf
        }
    }

    /// Extracts a sequence of velocities from a sequence of key events.
    pub struct Velocity<K>
    where
        K: SigT<Item = KeyEvents>,
    {
        sig_shared: SigShared<K>,
        held_velocity_by_note: Vec<(crate::Note, f32)>,
        last_velocity: f32,
        buf: Vec<f32>,
    }

    impl<K> Velocity<K>
    where
        K: SigT<Item = KeyEvents>,
    {
        pub fn new(sig_shared: SigShared<K>) -> Self {
            Self {
                sig_shared,
                held_velocity_by_note: Vec::new(),
                last_velocity: 0.0,
                buf: Vec::new(),
            }
        }
    }

    impl<K> SigT for Velocity<K>
    where
        K: SigT<Item = KeyEvents>,
    {
        type Item = f32;

        fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
            self.buf.clear();
            self.sig_shared.for_each_item(ctx, |events| {
                for event in events.iter() {
                    // Remove the held note if it already exists. This assumes there are no duplicate
                    // note in the vector.
                    for (i, (note, _)) in
                        self.held_velocity_by_note.iter().enumerate()
                    {
                        if *note == event.note {
                            self.held_velocity_by_note.remove(i);
                            break;
                        }
                    }
                    if event.pressed {
                        self.held_velocity_by_note
                            .push((event.note, event.velocity_01));
                    }
                    self.last_velocity = event.velocity_01;
                }
                // Yield the most-recently pressed note of all held notes, or the last touched note
                // if no notes are currently held.
                self.buf.push(
                    if let Some((_, velocity)) =
                        self.held_velocity_by_note.last()
                    {
                        *velocity
                    } else {
                        self.last_velocity
                    },
                )
            });
            &self.buf
        }
    }
}
