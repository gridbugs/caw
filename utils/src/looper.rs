use caw_core::{Buf, Sig, SigCtx, SigT};
use itertools::izip;

struct SequenceLooper<X, S, T, R, N>
where
    X: Clone,
    S: SigT<Item = Option<X>>,
    T: SigT<Item = bool>,
    R: SigT<Item = bool>,
    N: SigT<Item = u32>,
{
    sig: S,
    tick: T,
    recording: R,
    length: N,
    sequence: Vec<S::Item>,
    buf: Vec<S::Item>,
}

impl<X, S, T, R, N> SigT for SequenceLooper<X, S, T, R, N>
where
    X: Clone,
    S: SigT<Item = Option<X>>,
    T: SigT<Item = bool>,
    R: SigT<Item = bool>,
    N: SigT<Item = u32>,
{
    type Item = S::Item;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize_with(ctx.num_samples, || None);
        let sig = self.sig.sample(ctx);
        let tick = self.tick.sample(ctx);
        let recording = self.recording.sample(ctx);
        let length = self.length.sample(ctx);
        for (out, sample, tick, recording, length) in izip! {
            self.buf.iter_mut(),
            sig.iter(),
            tick.iter(),
            recording.iter(),
            length.iter(),
        } {
            *out = sample;
        }
        &self.buf
    }
}

pub fn sequence_looper<X: Clone>(
    sig: impl SigT<Item = Option<X>>,
    tick: impl SigT<Item = bool>,
    recording: impl SigT<Item = bool>,
    length: impl SigT<Item = u32>,
) -> Sig<impl SigT<Item = Option<X>>> {
    Sig(SequenceLooper {
        sig,
        tick,
        recording,
        length,
        sequence: Vec::new(),
        buf: Vec::new(),
    })
}
