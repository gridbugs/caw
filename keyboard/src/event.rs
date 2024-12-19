use crate::{
    chord::{Chord, Inversion},
    polyphony, MonoVoice, Note,
};
use caw_core_next::{FrameSig, FrameSigT, SigCtx};
use smallvec::{smallvec, SmallVec};
use std::collections::HashSet;

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
/// to group them into a collection because multiple key events may occur during the same frame.
/// This collection only uses the heap when more than one event occurred on the same sample which
/// is very unlikely.
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

    pub fn extend(&mut self, i: impl IntoIterator<Item = KeyEvent>) {
        self.0.extend(i);
    }
}

impl IntoIterator for KeyEvents {
    type Item = KeyEvent;

    type IntoIter = smallvec::IntoIter<[KeyEvent; 1]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<KeyEvent> for KeyEvents {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = KeyEvent>,
    {
        let mut events = Self::empty();
        for event in iter {
            events.push(event);
        }
        events
    }
}

pub trait KeyEventsT {
    fn merge<S>(self, other: S) -> FrameSig<impl FrameSigT<Item = KeyEvents>>
    where
        S: FrameSigT<Item = KeyEvents>;
    fn mono_voice(self) -> MonoVoice<impl FrameSigT<Item = KeyEvents>>;
    fn poly_voices(
        self,
        n: usize,
    ) -> Vec<MonoVoice<impl FrameSigT<Item = KeyEvents>>>;
}

impl<K> KeyEventsT for FrameSig<K>
where
    K: FrameSigT<Item = KeyEvents>,
{
    fn merge<S>(
        mut self,
        mut other: S,
    ) -> FrameSig<impl FrameSigT<Item = KeyEvents>>
    where
        S: FrameSigT<Item = KeyEvents>,
    {
        FrameSig::from_fn(move |ctx| {
            let mut s = self.frame_sample(ctx);
            let o = other.frame_sample(ctx);
            s.extend(o);
            s
        })
    }

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

impl FrameSigT for Inversion {
    type Item = Inversion;

    fn frame_sample(&mut self, _ctx: &SigCtx) -> Self::Item {
        *self
    }
}

pub struct ChordVoiceConfig<V, I>
where
    V: FrameSigT<Item = f32>,
    I: FrameSigT<Item = Inversion>,
{
    pub velocity_01: V,
    pub inversion: I,
}

impl<V, I> ChordVoiceConfig<V, I>
where
    V: FrameSigT<Item = f32>,
    I: FrameSigT<Item = Inversion>,
{
    pub fn with_velocity_01<V_>(
        self,
        velocity_01: V_,
    ) -> ChordVoiceConfig<V_, I>
    where
        V_: FrameSigT<Item = f32>,
    {
        let Self { inversion, .. } = self;
        ChordVoiceConfig {
            velocity_01,
            inversion,
        }
    }

    pub fn with_inversion<I_>(self, inversion: I_) -> ChordVoiceConfig<V, I_>
    where
        I_: FrameSigT<Item = Inversion>,
    {
        let Self { velocity_01, .. } = self;
        ChordVoiceConfig {
            velocity_01,
            inversion,
        }
    }
}

impl Default for ChordVoiceConfig<f32, Inversion> {
    fn default() -> Self {
        ChordVoiceConfig {
            velocity_01: 1.0,
            inversion: Inversion::default(),
        }
    }
}

fn key_events_from_chords<C, V, I>(
    chords: C,
    mut config: ChordVoiceConfig<V, I>,
) -> FrameSig<impl FrameSigT<Item = KeyEvents>>
where
    C: FrameSigT<Item = Option<Chord>>,
    V: FrameSigT<Item = f32>,
    I: FrameSigT<Item = Inversion>,
{
    let mut notes_to_release = HashSet::new();
    let mut pressed_notes = HashSet::new();
    FrameSig(chords).map_ctx(move |chord_opt, ctx| {
        let mut key_events = KeyEvents::empty();
        let inversion = config.inversion.frame_sample(ctx);
        if let Some(chord) = chord_opt {
            notes_to_release.clear();
            notes_to_release.clone_from(&pressed_notes);
            chord.with_notes(inversion, |note| {
                if pressed_notes.insert(note) {
                    let velocity_01 = config.velocity_01.frame_sample(ctx);
                    key_events.push(KeyEvent {
                        note,
                        velocity_01,
                        pressed: true,
                    });
                }
                notes_to_release.remove(&note);
            });
            for note in &notes_to_release {
                pressed_notes.remove(note);
                key_events.push(KeyEvent {
                    note: *note,
                    velocity_01: 0.0,
                    pressed: false,
                });
            }
        } else {
            for note in pressed_notes.drain() {
                key_events.push(KeyEvent {
                    note,
                    velocity_01: 0.0,
                    pressed: false,
                });
            }
        }
        key_events
    })
}

pub trait ChordsT {
    fn key_events<V, I>(
        self,
        config: ChordVoiceConfig<V, I>,
    ) -> FrameSig<impl FrameSigT<Item = KeyEvents>>
    where
        V: FrameSigT<Item = f32>,
        I: FrameSigT<Item = Inversion>;
}

impl<C> ChordsT for FrameSig<C>
where
    C: FrameSigT<Item = Option<Chord>>,
{
    fn key_events<V, I>(
        self,
        config: ChordVoiceConfig<V, I>,
    ) -> FrameSig<impl FrameSigT<Item = KeyEvents>>
    where
        V: FrameSigT<Item = f32>,
        I: FrameSigT<Item = Inversion>,
    {
        key_events_from_chords(self, config)
    }
}
