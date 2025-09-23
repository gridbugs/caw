use caw_builder_proc_macros::builder;
use caw_core::{Buf, Sig, SigCtx, SigT};
use caw_persistent::PersistentWitness;
use itertools::izip;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Sequence<T> {
    sequence: Vec<T>,
    index: usize,
}

impl<T> Sequence<T> {
    fn tick(&mut self) {
        assert!(self.sequence.len() > 0);
        self.index = (self.index + 1) % self.sequence.len();
    }

    fn current(&self) -> &T {
        &self.sequence[self.index]
    }

    fn current_mut(&mut self) -> &mut T {
        &mut self.sequence[self.index]
    }

    fn resize_with<F: FnMut() -> T>(&mut self, length: usize, f: F) {
        if length != self.sequence.len() {
            let length = length.max(1);
            self.sequence.resize_with(length, f);
            if self.index >= length {
                self.index = 0;
            }
        }
    }

    fn new_with<F: FnMut() -> T>(length: usize, f: F) -> Self {
        let length = length.max(1);
        let mut sequence = Vec::new();
        sequence.resize_with(length, f);
        Self { sequence, index: 0 }
    }
}

struct KeyLooperPersistentWitness;
impl<T> PersistentWitness<Sequence<Option<T>>> for KeyLooperPersistentWitness
where
    T: Serialize + for<'a> Deserialize<'a>,
{
    const NAME: &'static str = "key_looper";
}

pub trait KeyLooperPersist<T> {
    fn load(&self) -> Sequence<Option<T>>;
    fn save(&self, sequence: &Sequence<Option<T>>);
}

pub struct KeyLooperPersistNone;
impl<T> KeyLooperPersist<T> for KeyLooperPersistNone {
    fn load(&self) -> Sequence<Option<T>> {
        Sequence::new_with(1, || None)
    }

    fn save(&self, _sequence: &Sequence<Option<T>>) {}
}

pub struct KeyLooperPersistWithName(pub String);
impl<T> KeyLooperPersist<T> for KeyLooperPersistWithName
where
    T: Serialize + for<'a> Deserialize<'a>,
{
    fn load(&self) -> Sequence<Option<T>> {
        if let Some(sequence) = KeyLooperPersistentWitness.load_(&self.0) {
            sequence
        } else {
            Sequence::new_with(1, || None)
        }
    }

    fn save(&self, sequence: &Sequence<Option<T>>) {
        KeyLooperPersistentWitness.save_(sequence, &self.0)
    }
}

pub struct KeyLooper<X, S, T, C, N, P>
where
    X: Clone,
    S: SigT<Item = Option<X>>,
    T: SigT<Item = bool>,
    C: SigT<Item = bool>,
    N: SigT<Item = u32>,
    P: KeyLooperPersist<X>,
{
    sig: S,
    last_value: Option<X>,
    tick: T,
    clearing: C,
    length: N,
    sequence: Sequence<S::Item>,
    buf: Vec<S::Item>,
    persist: P,
}

impl<X, S, T, C, N, P> SigT for KeyLooper<X, S, T, C, N, P>
where
    X: Clone,
    S: SigT<Item = Option<X>>,
    T: SigT<Item = bool>,
    C: SigT<Item = bool>,
    N: SigT<Item = u32>,
    P: KeyLooperPersist<X>,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.clear();
        self.buf.resize_with(ctx.num_samples, || None);
        let sig = self.sig.sample(ctx);
        let tick = self.tick.sample(ctx);
        let clearing = self.clearing.sample(ctx);
        let length = self.length.sample(ctx);
        let mut changed_this_frame = false;
        for (out, sample, tick, clearing, length) in izip! {
            self.buf.iter_mut(),
            sig.iter(),
            tick.iter(),
            clearing.iter(),
            length.iter(),
        } {
            self.sequence.resize_with(length as usize, || None);
            if tick {
                self.sequence.tick();
                self.last_value = None;
            } else if sample.is_some() {
                self.last_value = sample.clone();
            }
            if clearing {
                *self.sequence.current_mut() = None;
                changed_this_frame = true;
            } else if self.last_value.is_some() {
                *self.sequence.current_mut() = self.last_value.clone();
                changed_this_frame = true;
            }
            *out = self.sequence.current().clone();
        }
        if changed_this_frame {
            self.persist.save(&self.sequence);
        }
        &self.buf
    }
}

impl<X, S, T, C, N, P> KeyLooper<X, S, T, C, N, P>
where
    X: Clone,
    S: SigT<Item = Option<X>>,
    T: SigT<Item = bool>,
    C: SigT<Item = bool>,
    N: SigT<Item = u32>,
    P: KeyLooperPersist<X>,
{
    fn new(sig: S, tick: T, clearing: C, length: N, persist: P) -> Sig<Self> {
        Sig(KeyLooper {
            sig,
            last_value: None,
            tick,
            clearing,
            length,
            sequence: persist.load(),
            buf: Vec::new(),
            persist,
        })
    }
}

builder! {
    #[constructor = "key_looper"]
    #[constructor_doc = "A looper for key presses"]
    #[generic_setter_type_name = "X"]
    #[build_fn = "KeyLooper::new"]
    #[build_ty = "Sig<KeyLooper<V, S, T, C, N, P>>"]
    #[extra_generic("V", "Clone")]
    pub struct KeyLooperBuilder {
        #[generic_with_constraint = "SigT<Item = Option<V>>"]
        #[generic_name = "S"]
        sig: _,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "T"]
        trig: _,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "C"]
        #[default = false]
        clearing: bool,
        #[generic_with_constraint = "SigT<Item = u32>"]
        #[generic_name = "N"]
        #[default = 16]
        length: u32,
        #[generic_with_constraint = "KeyLooperPersist<V>"]
        #[default = KeyLooperPersistNone]
        #[generic_name = "P"]
        persist: KeyLooperPersistNone,
    }
}

impl<X, S, T, C, N, P> KeyLooperBuilder<X, S, T, C, N, P>
where
    X: Clone + Serialize + for<'a> Deserialize<'a>,
    S: SigT<Item = Option<X>>,
    T: SigT<Item = bool>,
    C: SigT<Item = bool>,
    N: SigT<Item = u32>,
    P: KeyLooperPersist<X>,
{
    pub fn name(
        self,
        name: impl AsRef<str>,
    ) -> KeyLooperBuilder<X, S, T, C, N, KeyLooperPersistWithName> {
        let Self {
            sig,
            trig,
            clearing,
            length,
            ..
        } = self;
        KeyLooperBuilder {
            sig,
            trig,
            clearing,
            length,
            persist: KeyLooperPersistWithName(name.as_ref().to_string()),
        }
    }
}

pub struct ValueLooper<S, T, R, N>
where
    S: SigT,
    S::Item: Clone,
    T: SigT<Item = bool>,
    R: SigT<Item = bool>,
    N: SigT<Item = u32>,
{
    sig: S,
    tick: T,
    recording: R,
    length: N,
    sequence: Sequence<Option<S::Item>>,
    buf: Vec<S::Item>,
}

impl<S, T, R, N> SigT for ValueLooper<S, T, R, N>
where
    S: SigT,
    S::Item: Clone,
    T: SigT<Item = bool>,
    R: SigT<Item = bool>,
    N: SigT<Item = u32>,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.clear();
        let sig = self.sig.sample(ctx);
        let tick = self.tick.sample(ctx);
        let recording = self.recording.sample(ctx);
        let length = self.length.sample(ctx);
        for (sample, tick, recording, length) in izip! {
            sig.iter(),
            tick.iter(),
            recording.iter(),
            length.iter(),
        } {
            self.sequence.resize_with(length as usize, || None);
            if tick {
                self.sequence.tick();
            }
            let stored = self.sequence.current_mut();
            let out = match (recording, stored.clone()) {
                (true, _) | (_, None) => {
                    *stored = Some(sample.clone());
                    sample
                }
                (_, Some(stored)) => stored,
            };
            self.buf.push(out);
        }
        &self.buf
    }
}

impl<S, T, R, N> ValueLooper<S, T, R, N>
where
    S: SigT,
    S::Item: Clone,
    T: SigT<Item = bool>,
    R: SigT<Item = bool>,
    N: SigT<Item = u32>,
{
    fn new(sig: S, tick: T, recording: R, length: N) -> Sig<Self> {
        Sig(ValueLooper {
            sig,
            tick,
            recording,
            length,
            sequence: Sequence::new_with(1, || None),
            buf: Vec::new(),
        })
    }
}

builder! {
    #[constructor = "value_looper"]
    #[constructor_doc = "A looper for values such as knob positions"]
    #[generic_setter_type_name = "X"]
    #[build_fn = "ValueLooper::new"]
    #[build_ty = "Sig<ValueLooper<S, T, R, N>>"]
    pub struct ValueLooperBuilder {
        #[generic_with_constraint = "SigT"]
        #[generic_name = "S"]
        sig: _,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "T"]
        trig: _,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "R"]
        recording: _,
        #[generic_with_constraint = "SigT<Item = u32>"]
        #[generic_name = "N"]
        #[default = 16]
        length: u32,
    }
}
