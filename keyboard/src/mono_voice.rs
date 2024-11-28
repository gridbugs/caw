use crate::KeyEvents;
use caw_core_next::{
    frame_sig_shared, FrameSig, FrameSigShared, FrameSigT, SigCtx,
};

/// A collection of signals associated with a monophonic keyboard voice
pub struct MonoVoice<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    /// Stream of the current or most recent note played. This always yields a note even if no key
    /// is currently down as the sound of the note may still be playing, such as when an envelope
    /// is not finished releasing. The note yielded before any key has been pressed is arbitrary.
    pub note: FrameSig<Note<K>>,
    /// How hard the most recently pressed/released key was pressed/released
    pub velocity_01: FrameSig<Velocity01<K>>,
    /// True the entire time at least one key is held
    pub key_down_gate: FrameSig<KeyDownGate<K>>,
    /// True on the first audio sample after any key is pressed
    pub key_press_trigger: FrameSig<KeyPressTrigger<K>>,
}

impl<K> MonoVoice<FrameSigShared<K>>
where
    K: FrameSigT<Item = KeyEvents>,
{
    pub fn from_key_events(key_events: K) -> Self {
        let key_events_shared = frame_sig_shared(key_events).0;
        Self {
            note: FrameSig(Note::new(key_events_shared.clone())),
            velocity_01: FrameSig(Velocity01::new(key_events_shared.clone())),
            key_down_gate: FrameSig(KeyDownGate::new(
                key_events_shared.clone(),
            )),
            key_press_trigger: FrameSig(KeyPressTrigger::new(
                key_events_shared.clone(),
            )),
        }
    }
}

/// Extracts a sequence of notes from a sequence of key events.
pub struct Note<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    sig: K,
    held_notes: Vec<crate::Note>,
    last_note: crate::Note,
}

impl<K> Note<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    pub fn new(sig: K) -> Self {
        Self {
            sig,
            held_notes: Vec::new(),
            last_note: crate::Note::C4, // arbitrarily default to middle C
        }
    }
}

impl<K> FrameSigT for Note<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    type Item = crate::Note;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        let events = self.sig.frame_sample(ctx);
        for event in events.iter() {
            // Remove the held note if it already exists. This assumes there are no duplicate
            // notes in the vector.
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
        *self.held_notes.last().unwrap_or(&self.last_note)
    }
}

/// Extracts a sequence of velocities from a sequence of key events.
pub struct Velocity01<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    sig: K,
    held_velocity_by_note: Vec<(crate::Note, f32)>,
    last_velocity: f32,
}

impl<K> Velocity01<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    pub fn new(sig: K) -> Self {
        Self {
            sig,
            held_velocity_by_note: Vec::new(),
            last_velocity: 0.0,
        }
    }
}

impl<K> FrameSigT for Velocity01<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    type Item = f32;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        let events = self.sig.frame_sample(ctx);
        for event in events.iter() {
            // Remove the held note if it already exists. This assumes there are no duplicate
            // note in the vector.
            for (i, (note, _)) in self.held_velocity_by_note.iter().enumerate()
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
        if let Some((_, velocity)) = self.held_velocity_by_note.last() {
            *velocity
        } else {
            self.last_velocity
        }
    }
}

/// Extracts a key down gate from a sequence of key events.
pub struct KeyDownGate<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    sig: K,
    num_keys_down: usize,
}

impl<K> KeyDownGate<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    pub fn new(sig: K) -> Self {
        Self {
            sig,
            num_keys_down: 0,
        }
    }
}

impl<K> FrameSigT for KeyDownGate<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    type Item = bool;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        let events = self.sig.frame_sample(ctx);
        for event in events.iter() {
            if event.pressed {
                self.num_keys_down += 1;
            } else {
                self.num_keys_down -= 1;
            }
        }
        self.num_keys_down > 0
    }
}

/// Extracts a key press trigger from a sequence of key events.
pub struct KeyPressTrigger<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    sig: K,
}

impl<K> KeyPressTrigger<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    pub fn new(sig: K) -> Self {
        Self { sig }
    }
}

impl<K> FrameSigT for KeyPressTrigger<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    type Item = bool;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        let events = self.sig.frame_sample(ctx);
        for event in events.iter() {
            if event.pressed {
                return true;
            }
        }
        false
    }
}
