use crate::{BufferedSignal, SignalCtx, SignalTrait};
use std::{iter::Sum, ops::Add};

pub struct SignalAdd<L, R>
where
    L: SignalTrait,
    R: SignalTrait,
    L::Item: Clone,
    R::Item: Clone,
    L::Item: Add<R::Item>,
{
    lhs: BufferedSignal<L>,
    rhs: BufferedSignal<R>,
}

impl<L, R> SignalTrait for SignalAdd<L, R>
where
    L: SignalTrait,
    R: SignalTrait,
    L::Item: Clone,
    R::Item: Clone,
    L::Item: Add<R::Item>,
{
    type Item = <L::Item as Add<R::Item>>::Output;
    type SampleBuffer = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        self.lhs.sample_batch(ctx, n);
        self.rhs.sample_batch(ctx, n);
        for (lhs, rhs) in self.lhs.samples().zip(self.rhs.samples()) {
            sample_buffer.push(lhs.clone().add(rhs.clone()))
        }
    }
}

impl<S, R> Add<BufferedSignal<R>> for BufferedSignal<S>
where
    S: SignalTrait,
    R: SignalTrait,
    S::Item: Add<R::Item>,
    S::Item: Clone,
    R::Item: Clone,
{
    type Output = BufferedSignal<SignalAdd<S, R>>;

    fn add(self, rhs: BufferedSignal<R>) -> Self::Output {
        SignalAdd { lhs: self, rhs }.buffered()
    }
}

pub struct SignalSum<S>(Vec<BufferedSignal<S>>)
where
    S: SignalTrait,
    S::Item: Add;

impl<S> SignalTrait for SignalSum<S>
where
    S: SignalTrait<Item = f32>,
{
    type Item = f32;
    type SampleBuffer = Vec<Self::Item>;

    fn sample_batch(
        &mut self,
        ctx: &SignalCtx,
        n: usize,
        sample_buffer: &mut Self::SampleBuffer,
    ) {
        for _ in 0..n {
            sample_buffer.push(0.0);
        }
        for buffered_signal in &mut self.0 {
            buffered_signal.sample_batch(ctx, n);
            for (out, sample) in
                sample_buffer.iter_mut().zip(buffered_signal.samples())
            {
                *out += sample;
            }
        }
    }
}
impl<S> Sum<BufferedSignal<S>> for BufferedSignal<SignalSum<S>>
where
    S: SignalTrait<Item = f32>,
{
    fn sum<I: Iterator<Item = BufferedSignal<S>>>(iter: I) -> Self {
        SignalSum(iter.collect()).buffered()
    }
}
