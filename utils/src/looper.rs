use caw_core::{Buf, Sig, SigCtx, SigT};
use itertools::izip;

#[derive(Default)]
struct Sequence<T> {
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

struct KeyLooper<X, S, T, C, N>
where
    X: Clone,
    S: SigT<Item = Option<X>>,
    T: SigT<Item = bool>,
    C: SigT<Item = bool>,
    N: SigT<Item = u32>,
{
    sig: S,
    last_value: S::Item,
    tick: T,
    clearing: C,
    length: N,
    sequence: Sequence<S::Item>,
    buf: Vec<S::Item>,
}

impl<X, S, T, C, N> SigT for KeyLooper<X, S, T, C, N>
where
    X: Clone,
    S: SigT<Item = Option<X>>,
    T: SigT<Item = bool>,
    C: SigT<Item = bool>,
    N: SigT<Item = u32>,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.clear();
        self.buf.resize_with(ctx.num_samples, || None);
        let sig = self.sig.sample(ctx);
        let tick = self.tick.sample(ctx);
        let clearing = self.clearing.sample(ctx);
        let length = self.length.sample(ctx);
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
            } else if self.last_value.is_some() {
                *self.sequence.current_mut() = self.last_value.clone();
            }
            *out = self.sequence.current().clone();
        }
        &self.buf
    }
}

pub fn key_looper<X: Clone>(
    sig: impl SigT<Item = Option<X>>,
    tick: impl SigT<Item = bool>,
    clearing: impl SigT<Item = bool>,
    length: impl SigT<Item = u32>,
) -> Sig<impl SigT<Item = Option<X>>> {
    Sig(KeyLooper {
        sig,
        last_value: None,
        tick,
        clearing,
        length,
        sequence: Sequence::new_with(1, || None),
        buf: Vec::new(),
    })
}

struct ValueLooper<S, T, R, N>
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

pub fn value_looper<T: Clone>(
    sig: impl SigT<Item = T>,
    tick: impl SigT<Item = bool>,
    recording: impl SigT<Item = bool>,
    length: impl SigT<Item = u32>,
) -> Sig<impl SigT<Item = T>> {
    Sig(ValueLooper {
        sig,
        tick,
        recording,
        length,
        sequence: Sequence::new_with(1, || None),
        buf: Vec::new(),
    })
}
