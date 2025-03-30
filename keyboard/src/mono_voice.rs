use crate::KeyEvents;
use caw_core::{sig_shared, Buf, Sig, SigCtx, SigShared, SigT};
use itertools::izip;

/// A collection of signals associated with a monophonic keyboard voice
pub struct MonoVoice<K>
where
    K: SigT<Item = KeyEvents>,
{
    /// Stream of the current or most recent note played. This always yields a note even if no key
    /// is currently down as the sound of the note may still be playing, such as when an envelope
    /// is not finished releasing. The note yielded before any key has been pressed is arbitrary.
    pub note: Sig<Note<K>>,
    /// How hard the most recently pressed/released key was pressed/released
    pub velocity_01: Sig<Velocity01<K>>,
    /// True the entire time at least one key is held
    pub key_down_gate: Sig<KeyDownGate<K>>,
    /// True on the first audio sample after any key is pressed
    pub key_press_trig: Sig<KeyPressTrig_<K>>,
}

impl<K> MonoVoice<SigShared<K>>
where
    K: SigT<Item = KeyEvents>,
{
    pub fn from_key_events(key_events: K) -> Self {
        let key_events_shared = sig_shared(key_events).0;
        Self {
            note: Sig(Note::new(key_events_shared.clone())),
            velocity_01: Sig(Velocity01::new(key_events_shared.clone())),
            key_down_gate: Sig(KeyDownGate::new(key_events_shared.clone())),
            key_press_trig: Sig(KeyPressTrig_::new(key_events_shared.clone())),
        }
    }
}

/// Extracts a sequence of notes from a sequence of key events.
pub struct Note<K>
where
    K: SigT<Item = KeyEvents>,
{
    sig: K,
    held_notes: Vec<crate::Note>,
    last_note: crate::Note,
    buf: Vec<crate::Note>,
}

impl<K> Note<K>
where
    K: SigT<Item = KeyEvents>,
{
    pub fn new(sig: K) -> Self {
        Self {
            sig,
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
        self.buf.resize(ctx.num_samples, crate::Note::C4);
        let events = self.sig.sample(ctx);
        for (out, events) in izip! {
            self.buf.iter_mut(),
            events.iter(),
        } {
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
            *out = *self.held_notes.last().unwrap_or(&self.last_note);
        }
        &self.buf
    }
}

/// Extracts a sequence of velocities from a sequence of key events.
pub struct Velocity01<K>
where
    K: SigT<Item = KeyEvents>,
{
    sig: K,
    held_velocity_by_note: Vec<(crate::Note, f32)>,
    last_velocity: f32,
    buf: Vec<f32>,
}

impl<K> Velocity01<K>
where
    K: SigT<Item = KeyEvents>,
{
    pub fn new(sig: K) -> Self {
        Self {
            sig,
            held_velocity_by_note: Vec::new(),
            last_velocity: 0.0,
            buf: Vec::new(),
        }
    }
}

impl<K> SigT for Velocity01<K>
where
    K: SigT<Item = KeyEvents>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        let events = self.sig.sample(ctx);
        for (out, events) in izip! { self.buf.iter_mut(), events.iter() } {
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
                    self.last_velocity = event.velocity_01;
                }
            }
            // Yield the most-recently pressed note of all held notes, or the last touched note
            // if no notes are currently held.
            *out = if let Some((_, velocity)) =
                self.held_velocity_by_note.last()
            {
                *velocity
            } else {
                self.last_velocity
            };
        }
        &self.buf
    }
}

/// Extracts a key down gate from a sequence of key events.
pub struct KeyDownGate<K>
where
    K: SigT<Item = KeyEvents>,
{
    sig: K,
    num_keys_down: usize,
    buf: Vec<bool>,
}

impl<K> KeyDownGate<K>
where
    K: SigT<Item = KeyEvents>,
{
    pub fn new(sig: K) -> Self {
        Self {
            sig,
            num_keys_down: 0,
            buf: Vec::new(),
        }
    }
}

impl<K> SigT for KeyDownGate<K>
where
    K: SigT<Item = KeyEvents>,
{
    type Item = bool;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, false);
        let events = self.sig.sample(ctx);
        for (out, events) in izip! { self.buf.iter_mut(), events.iter() } {
            for event in events.iter() {
                if event.pressed {
                    self.num_keys_down += 1;
                } else {
                    self.num_keys_down -= 1;
                }
            }
            *out = self.num_keys_down > 0;
        }
        &self.buf
    }
}

/// Extracts a key press trigger from a sequence of key events.
pub struct KeyPressTrig_<K>
where
    K: SigT<Item = KeyEvents>,
{
    sig: K,
    buf: Vec<bool>,
}

impl<K> KeyPressTrig_<K>
where
    K: SigT<Item = KeyEvents>,
{
    pub fn new(sig: K) -> Self {
        Self {
            sig,
            buf: Vec::new(),
        }
    }
}

impl<K> SigT for KeyPressTrig_<K>
where
    K: SigT<Item = KeyEvents>,
{
    type Item = bool;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, false);
        let events = self.sig.sample(ctx);
        'outer: for (out, events) in
            izip! { self.buf.iter_mut(), events.iter() }
        {
            for event in events.iter() {
                if event.pressed {
                    *out = true;
                    continue 'outer;
                }
            }
            *out = false;
        }
        &self.buf
    }
}
