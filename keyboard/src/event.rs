use crate::{
    chord::{Chord, Inversion},
    polyphony, MonoVoice, MonoVoice_, Note,
};
use caw_core::{Buf, ConstBuf, FrameSig, FrameSigT, Sig, SigCtx, SigT};
use itertools::izip;
use rand::{rngs::StdRng, Rng, SeedableRng};
use smallvec::{smallvec, SmallVec};
use std::{cmp::Ordering, collections::HashSet};

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
#[derive(Clone, Debug, Default)]
pub struct KeyEvents(SmallVec<[KeyEvent; 4]>);

impl KeyEvents {
    pub fn empty() -> Self {
        Self(smallvec![])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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

    type IntoIter = smallvec::IntoIter<[KeyEvent; 4]>;

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

pub trait KeyEventsT_ {
    fn merge<S>(self, other: S) -> Sig<impl SigT<Item = KeyEvents>>
    where
        S: SigT<Item = KeyEvents>;

    fn mono_voice(self) -> MonoVoice_<impl SigT<Item = KeyEvents>>;

    fn poly_voices(
        self,
        n: usize,
    ) -> Vec<MonoVoice_<impl SigT<Item = KeyEvents>>>;

    fn arp<G, V, H, L, S>(
        self,
        gate: G,
        config: ArpConfig_<V, H, L, S>,
    ) -> Sig<impl SigT<Item = KeyEvents>>
    where
        G: SigT<Item = bool>,
        V: SigT<Item = f32>,
        H: SigT<Item = u32>,
        L: SigT<Item = u32>,
        S: SigT<Item = ArpShape>;
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

    fn arp<G, V, H, L, S>(
        self,
        gate: G,
        config: ArpConfig<V, H, L, S>,
    ) -> FrameSig<impl FrameSigT<Item = KeyEvents>>
    where
        G: FrameSigT<Item = bool>,
        V: FrameSigT<Item = f32>,
        H: FrameSigT<Item = u32>,
        L: FrameSigT<Item = u32>,
        S: FrameSigT<Item = ArpShape>;
}

impl<K> KeyEventsT_ for Sig<K>
where
    K: SigT<Item = KeyEvents>,
{
    fn merge<S>(mut self, mut other: S) -> Sig<impl SigT<Item = KeyEvents>>
    where
        S: SigT<Item = KeyEvents>,
    {
        Sig::from_buf_fn(move |ctx, buf: &mut Vec<KeyEvents>| {
            buf.clear();
            let s = self.sample(ctx);
            let o = other.sample(ctx);
            for (mut s, o) in izip! { s.iter(), o.iter() } {
                s.extend(o);
                buf.push(s);
            }
        })
    }

    fn mono_voice(self) -> MonoVoice_<impl SigT<Item = KeyEvents>> {
        MonoVoice_::from_key_events(self.0)
    }

    fn poly_voices(
        self,
        n: usize,
    ) -> Vec<MonoVoice_<impl SigT<Item = KeyEvents>>> {
        polyphony::voices_from_key_events_(self.0, n)
    }

    fn arp<G, V, H, L, S>(
        self,
        gate: G,
        config: ArpConfig_<V, H, L, S>,
    ) -> Sig<impl SigT<Item = KeyEvents>>
    where
        G: SigT<Item = bool>,
        V: SigT<Item = f32>,
        H: SigT<Item = u32>,
        L: SigT<Item = u32>,
        S: SigT<Item = ArpShape>,
    {
        key_events_from_chords_arp_(self, gate, config)
    }
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

    fn arp<G, V, H, L, S>(
        self,
        gate: G,
        config: ArpConfig<V, H, L, S>,
    ) -> FrameSig<impl FrameSigT<Item = KeyEvents>>
    where
        G: FrameSigT<Item = bool>,
        V: FrameSigT<Item = f32>,
        H: FrameSigT<Item = u32>,
        L: FrameSigT<Item = u32>,
        S: FrameSigT<Item = ArpShape>,
    {
        key_events_from_chords_arp(self, gate, config)
    }
}

impl FrameSigT for Inversion {
    type Item = Inversion;

    fn frame_sample(&mut self, _ctx: &SigCtx) -> Self::Item {
        *self
    }
}

impl SigT for Inversion {
    type Item = Inversion;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            count: ctx.num_samples,
            value: *self,
        }
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

#[derive(Default, Debug, Clone)]
pub enum ArpShape {
    #[default]
    Up,
    Down,
    UpDown,
    DownUp,
    Random,
    Indices(Vec<Option<usize>>),
}

impl SigT for ArpShape {
    type Item = ArpShape;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            count: ctx.num_samples,
            value: self.clone(),
        }
    }
}

impl FrameSigT for ArpShape {
    type Item = ArpShape;

    fn frame_sample(&mut self, _ctx: &SigCtx) -> Self::Item {
        self.clone()
    }
}

#[derive(Debug)]
struct ArpNoteStoreEntry {
    note: Note,
    count: usize,
}

#[derive(Default, Debug)]
struct ArpNoteStore {
    entries: Vec<ArpNoteStoreEntry>,
}

impl ArpNoteStore {
    /// Insert a new note unless it's already present, preserving the sortedness of the notes
    /// vector. Returns the index of the newly added note.
    fn insert(&mut self, note_to_insert: Note) -> Option<usize> {
        for (i, entry) in self.entries.iter_mut().enumerate() {
            match entry.note.cmp(&note_to_insert) {
                Ordering::Greater => {
                    self.entries.insert(
                        i,
                        ArpNoteStoreEntry {
                            note: note_to_insert,
                            count: 1,
                        },
                    );
                    return Some(i);
                }
                Ordering::Equal => {
                    // The note is already present. Update the count.
                    entry.count += 1;
                    return None;
                }
                Ordering::Less => (),
            }
        }
        self.entries.push(ArpNoteStoreEntry {
            note: note_to_insert,
            count: 1,
        });
        Some(self.entries.len() - 1)
    }

    /// Remove a note if it's present, preserving the sortedness of the notes vector. Returns the
    /// index of the removed note.
    fn remove(&mut self, note_to_remove: Note) -> Option<usize> {
        for (i, entry) in self.entries.iter_mut().enumerate() {
            match entry.note.cmp(&note_to_remove) {
                Ordering::Greater => break,
                Ordering::Equal => {
                    entry.count -= 1;
                    if entry.count == 0 {
                        self.entries.remove(i);
                        return Some(i);
                    } else {
                        return None;
                    }
                }
                Ordering::Less => (),
            }
        }
        None
    }
}

#[derive(Debug)]
struct ArpState {
    store: ArpNoteStore,
    index: usize,
    current_note: Option<Note>,
    ascending: bool,
    rng: StdRng,
}

impl ArpState {
    fn new() -> Self {
        Self {
            store: ArpNoteStore::default(),
            index: 0,
            current_note: None,
            ascending: true,
            rng: StdRng::from_entropy(),
        }
    }

    fn insert_note(&mut self, note: Note) {
        if let Some(index) = self.store.insert(note) {
            if self.index > index {
                self.index += 1;
            }
        }
    }
    fn remove_note(&mut self, note: Note) {
        if let Some(index) = self.store.remove(note) {
            if self.index > index {
                self.index -= 1;
            }
        }
    }
    fn reset(&mut self, shape: ArpShape) {
        self.index = 0;
        use ArpShape::*;
        match shape {
            Up | UpDown => {
                self.ascending = true;
            }
            Down | DownUp => {
                self.ascending = false;
            }
            Random | Indices(_) => (),
        }
    }
    fn tick_up(&mut self) -> Note {
        if self.index >= self.store.entries.len() {
            self.index = 0;
        }
        let entry = &self.store.entries[self.index];
        self.index += 1;
        entry.note
    }
    fn tick_down(&mut self) -> Note {
        if self.index == 0 {
            self.index = self.store.entries.len();
        }
        self.index -= 1;
        self.store.entries[self.index].note
    }
    fn tick(&mut self, shape: ArpShape) {
        self.current_note = if self.store.entries.is_empty() {
            self.reset(shape);
            None
        } else {
            use ArpShape::*;
            let note = match shape {
                Up => self.tick_up(),
                Down => self.tick_down(),
                UpDown | DownUp => {
                    if self.ascending {
                        let note = self.tick_up();
                        if self.index >= self.store.entries.len() {
                            self.ascending = false;
                            self.index = self.store.entries.len() - 1;
                        }
                        note
                    } else {
                        let note = self.tick_down();
                        if self.index == 0 {
                            self.ascending = true;
                            self.index = 1;
                        }
                        note
                    }
                }
                Random => {
                    let index = self.rng.gen_range(0..self.store.entries.len());
                    self.store.entries[index].note
                }
                Indices(indices) => {
                    if indices.is_empty() {
                        self.current_note = None;
                        return;
                    }
                    if self.index >= indices.len() {
                        self.index = 0;
                    }
                    self.current_note = indices[self.index].and_then(|i| {
                        self.store.entries.get(i).map(|e| e.note)
                    });
                    self.index += 1;
                    return;
                }
            };
            Some(note)
        };
    }
}

pub struct ArpConfig_<V, H, L, S>
where
    V: SigT<Item = f32>,
    H: SigT<Item = u32>,
    L: SigT<Item = u32>,
    S: SigT<Item = ArpShape>,
{
    pub velocity_01: V,
    pub extend_octaves_high: H,
    pub extend_octaves_low: L,
    pub shape: S,
}

pub struct ArpConfig<V, H, L, S>
where
    V: FrameSigT<Item = f32>,
    H: FrameSigT<Item = u32>,
    L: FrameSigT<Item = u32>,
    S: FrameSigT<Item = ArpShape>,
{
    pub velocity_01: V,
    pub extend_octaves_high: H,
    pub extend_octaves_low: L,
    pub shape: S,
}

impl Default for ArpConfig<f32, u32, u32, ArpShape> {
    fn default() -> Self {
        ArpConfig {
            velocity_01: 0.0,
            extend_octaves_high: 0,
            extend_octaves_low: 0,
            shape: ArpShape::Up,
        }
    }
}

impl<V, H, L, S> ArpConfig<V, H, L, S>
where
    V: FrameSigT<Item = f32>,
    H: FrameSigT<Item = u32>,
    L: FrameSigT<Item = u32>,
    S: FrameSigT<Item = ArpShape>,
{
    pub fn with_velocity_01<V_>(self, velocity_01: V_) -> ArpConfig<V_, H, L, S>
    where
        V_: FrameSigT<Item = f32>,
    {
        let Self {
            extend_octaves_high,
            extend_octaves_low,
            shape,
            ..
        } = self;
        ArpConfig {
            velocity_01,
            extend_octaves_high,
            extend_octaves_low,
            shape,
        }
    }

    pub fn with_extend_octaves_high<H_>(
        self,
        extend_octaves_high: H_,
    ) -> ArpConfig<V, H_, L, S>
    where
        H_: FrameSigT<Item = u32>,
    {
        let Self {
            velocity_01,
            extend_octaves_low,
            shape,
            ..
        } = self;
        ArpConfig {
            velocity_01,
            extend_octaves_high,
            extend_octaves_low,
            shape,
        }
    }

    pub fn with_extend_octaves_low<L_>(
        self,
        extend_octaves_low: L_,
    ) -> ArpConfig<V, H, L_, S>
    where
        L_: FrameSigT<Item = u32>,
    {
        let Self {
            velocity_01,
            extend_octaves_high,
            shape,
            ..
        } = self;
        ArpConfig {
            velocity_01,
            extend_octaves_high,
            extend_octaves_low,
            shape,
        }
    }

    pub fn with_shape<S_>(self, shape: S_) -> ArpConfig<V, H, L, S_>
    where
        S_: FrameSigT<Item = ArpShape>,
    {
        let Self {
            velocity_01,
            extend_octaves_high,
            extend_octaves_low,
            ..
        } = self;
        ArpConfig {
            velocity_01,
            extend_octaves_high,
            extend_octaves_low,
            shape,
        }
    }
}

fn key_events_from_chords_arp_<K, G, V, H, L, S>(
    mut key_events: K,
    gate: G,
    mut config: ArpConfig_<V, H, L, S>,
) -> Sig<impl SigT<Item = KeyEvents>>
where
    K: SigT<Item = KeyEvents>,
    G: SigT<Item = bool>,
    V: SigT<Item = f32>,
    H: SigT<Item = u32>,
    L: SigT<Item = u32>,
    S: SigT<Item = ArpShape>,
{
    let mut state = ArpState::new();
    let mut trigger = Sig(gate).gate_to_trig_rising_edge();
    Sig::from_buf_fn(move |ctx, buf| {
        let trigger = trigger.sample(ctx);
        let key_events = key_events.sample(ctx);
        let extend_octaves_high = config.extend_octaves_high.sample(ctx);
        let extend_octaves_low = config.extend_octaves_low.sample(ctx);
        let shape = config.shape.sample(ctx);
        for (
            trigger,
            key_events,
            extend_octaves_high,
            extend_octaves_low,
            shape,
        ) in izip! {
            trigger.iter(),
            key_events.iter(),
            extend_octaves_high.iter(),
            extend_octaves_low.iter(),
            shape.iter(),
        } {
            let mut events = KeyEvents::empty();
            for event in key_events {
                if event.pressed {
                    state.insert_note(event.note);
                    for i in 0..extend_octaves_high {
                        if let Some(note) =
                            event.note.add_octaves_checked(i as i8 + 1)
                        {
                            state.insert_note(note);
                        }
                    }
                    for i in 0..extend_octaves_low {
                        if let Some(note) =
                            event.note.add_octaves_checked(-(i as i8 + 1))
                        {
                            state.insert_note(note);
                        }
                    }
                } else {
                    state.remove_note(event.note);
                    for i in 0..extend_octaves_high {
                        if let Some(note) =
                            event.note.add_octaves_checked(i as i8 + 1)
                        {
                            state.remove_note(note);
                        }
                    }
                    for i in 0..extend_octaves_low {
                        if let Some(note) =
                            event.note.add_octaves_checked(-(i as i8 + 1))
                        {
                            state.remove_note(note);
                        }
                    }
                }
            }
            if trigger {
                if let Some(note) = state.current_note {
                    events.push(KeyEvent {
                        note,
                        pressed: false,
                        velocity_01: 0.0,
                    });
                }
                state.tick(shape);
                if let Some(note) = state.current_note {
                    events.push(KeyEvent {
                        note,
                        pressed: true,
                        velocity_01: 1.0,
                    });
                }
            }
            buf.push(events);
        }
    })
}

fn key_events_from_chords_arp<K, G, V, H, L, S>(
    mut key_events: K,
    gate: G,
    mut config: ArpConfig<V, H, L, S>,
) -> FrameSig<impl FrameSigT<Item = KeyEvents>>
where
    K: FrameSigT<Item = KeyEvents>,
    G: FrameSigT<Item = bool>,
    V: FrameSigT<Item = f32>,
    H: FrameSigT<Item = u32>,
    L: FrameSigT<Item = u32>,
    S: FrameSigT<Item = ArpShape>,
{
    let mut state = ArpState::new();
    let mut gate = FrameSig(gate).edges();
    FrameSig::from_fn(move |ctx| {
        gate.frame_sample(ctx);
        let mut events = KeyEvents::empty();
        for event in key_events.frame_sample(ctx) {
            if event.pressed {
                state.insert_note(event.note);
                for i in 0..config.extend_octaves_high.frame_sample(ctx) {
                    if let Some(note) =
                        event.note.add_octaves_checked(i as i8 + 1)
                    {
                        state.insert_note(note);
                    }
                }
                for i in 0..config.extend_octaves_low.frame_sample(ctx) {
                    if let Some(note) =
                        event.note.add_octaves_checked(-(i as i8 + 1))
                    {
                        state.insert_note(note);
                    }
                }
            } else {
                state.remove_note(event.note);
                for i in 0..config.extend_octaves_high.frame_sample(ctx) {
                    if let Some(note) =
                        event.note.add_octaves_checked(i as i8 + 1)
                    {
                        state.remove_note(note);
                    }
                }
                for i in 0..config.extend_octaves_low.frame_sample(ctx) {
                    if let Some(note) =
                        event.note.add_octaves_checked(-(i as i8 + 1))
                    {
                        state.remove_note(note);
                    }
                }
            }
        }
        if gate.is_rising() {
            if let Some(note) = state.current_note {
                events.push(KeyEvent {
                    note,
                    pressed: false,
                    velocity_01: 0.0,
                });
            }
            state.tick(config.shape.frame_sample(ctx));
            if let Some(note) = state.current_note {
                events.push(KeyEvent {
                    note,
                    pressed: true,
                    velocity_01: 1.0,
                });
            }
        }
        events
    })
}
