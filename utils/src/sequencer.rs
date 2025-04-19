use caw_core::{Buf, Sig, SigCtx, SigT};

struct ValueSequencer<T, V>
where
    T: SigT<Item = bool>,
    V: Clone,
{
    trig: T,
    values: Vec<V>,
    buf: Vec<V>,
    index: usize,
}

impl<T, V> SigT for ValueSequencer<T, V>
where
    T: SigT<Item = bool>,
    V: Clone,
{
    type Item = V;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.clear();
        let trig = self.trig.sample(ctx);
        for trig in trig.iter() {
            if trig {
                self.index = (self.index + 1) % self.values.len();
            }
            self.buf.push(self.values[self.index].clone());
        }
        &self.buf
    }
}

pub fn value_sequencer<T, V>(
    trig: T,
    values: impl IntoIterator<Item = V>,
) -> Sig<impl SigT<Item = V>>
where
    T: SigT<Item = bool>,
    V: Clone,
{
    let values: Vec<_> = values.into_iter().collect();
    Sig(ValueSequencer {
        // Start at the end of the sequence so the first tick plays the
        // first note (if the sequence contains notes).
        index: values.len() - 1,
        trig,
        values,
        buf: Vec::new(),
    })
}
