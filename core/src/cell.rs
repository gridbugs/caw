use crate::{
    sig::{
        Buf, Sig, SigBoxed, SigCtx, SigSampleIntoBufT, SigShared, SigT,
        sig_boxed, sig_shared,
    },
    stereo::{Channel, Stereo, StereoPair},
};
use std::sync::{Arc, RwLock};

/// Private inner type of the `Cell_` type.
struct Unshared<T: Clone> {
    sig_boxed: Arc<RwLock<SigBoxed<T>>>,
    buf: Vec<T>,
}

impl<T: Clone> Clone for Unshared<T> {
    fn clone(&self) -> Self {
        Self {
            sig_boxed: Arc::clone(&self.sig_boxed),
            buf: Vec::new(),
        }
    }
}

impl<T: Clone> Unshared<T> {
    fn new<S>(sig: S) -> Self
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        Unshared {
            sig_boxed: Arc::new(RwLock::new(sig_boxed(sig).0)),
            buf: Vec::new(),
        }
    }

    fn set<S>(&self, sig: S)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        *self.sig_boxed.write().unwrap() = sig_boxed(sig).0;
    }
}

impl<T: Clone> SigT for Unshared<T> {
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let mut unlocked = self.sig_boxed.write().unwrap();
        unlocked.sample_into_buf(ctx, &mut self.buf);
        &self.buf
    }
}

impl<T: Clone> SigSampleIntoBufT for Unshared<T> {
    type Item = T;

    fn sample_into_buf(&mut self, ctx: &SigCtx, buf: &mut Vec<Self::Item>) {
        self.sig_boxed.write().unwrap().sample_into_buf(ctx, buf);
    }

    fn sample_into_slice(
        &mut self,
        ctx: &SigCtx,
        stride: usize,
        offset: usize,
        out: &mut [Self::Item],
    ) {
        self.sig_boxed
            .write()
            .unwrap()
            .sample_into_slice(ctx, stride, offset, out);
    }
}

pub fn cell_<S>(sig: S) -> Sig<Cell_<S::Item>>
where
    S: SigT + Sync + Send + 'static,
{
    Sig(Cell_::new(sig))
}

#[derive(Clone)]
pub struct Cell_<T: Clone>(Sig<SigShared<Unshared<T>>>);

impl<T: Clone> SigT for Cell_<T> {
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.0.sample(ctx)
    }
}

impl<T: Clone> Cell_<T> {
    pub fn new<S>(sig: S) -> Self
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        Self(sig_shared(Unshared::new(sig)))
    }

    pub fn set<S>(&self, sig: S)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        self.0.0.with_inner_mut(|unshared| unshared.set(sig));
    }
}

impl<T: Clone> Sig<Cell_<T>> {
    pub fn set<S>(&self, sig: S)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
    {
        self.0.set(sig);
    }
}

impl<T: Clone> SigSampleIntoBufT for Cell_<T> {
    type Item = T;

    fn sample_into_buf(&mut self, ctx: &SigCtx, buf: &mut Vec<Self::Item>) {
        self.0.sample_into_buf(ctx, buf);
    }

    fn sample_into_slice(
        &mut self,
        ctx: &SigCtx,
        stride: usize,
        offset: usize,
        out: &mut [Self::Item],
    ) {
        self.0.sample_into_slice(ctx, stride, offset, out);
    }
}

/// A container for a signal yielding a type `T` whose contentents (a signal) can be changed to a
/// different signal later. The type of the signal is erased (`T` is the type _yielded_ by the
/// signal) so signals of different types may be placed into the same cell. Call its `set` method
/// to update the signal contained in the cell.
pub type Cell<T> = Sig<Cell_<T>>;
pub type CellF32 = Cell<f32>;

pub fn cell<T>(
    initial_sig: impl SigT<Item = T> + Sync + Send + 'static,
) -> Cell<T>
where
    T: Clone,
{
    cell_(initial_sig)
}

pub fn cell_f32(
    initial_sig: impl SigT<Item = f32> + Sync + Send + 'static,
) -> CellF32 {
    cell_(initial_sig)
}

/// Create a new signal cell initialized with a constant signal yielding the default value of the
/// type `T`.
pub fn cell_default<T>() -> Cell<T>
where
    T: SigT<Item = T> + Clone + Default + Sync + Send + 'static,
{
    cell(T::default())
}

pub type StereoCell<T> = Stereo<Cell<T>, Cell<T>>;
pub type StereoCellF32 = StereoCell<f32>;

pub fn stereo_cell_fn<S, F>(mut f: F) -> StereoCell<S::Item>
where
    S: SigT + Sync + Send + 'static,
    F: FnMut() -> S,
{
    Stereo::new_fn(|| cell(f()))
}

pub fn stereo_cell_default<T>() -> StereoCell<T>
where
    T: SigT<Item = T> + Clone + Default + Sync + Send + 'static,
{
    stereo_cell_fn(T::default)
}

impl<T> StereoCell<T>
where
    T: Clone,
{
    pub fn set<F, S>(&self, mut f: F)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
        F: FnMut() -> S,
    {
        self.as_ref().with(|s| s.set(f()));
    }

    pub fn set_channel<F, S>(&self, mut f: F)
    where
        S: SigT<Item = T> + Sync + Send + 'static,
        F: FnMut(Channel) -> S,
    {
        self.as_ref().with_channel(|channel, s| s.set(f(channel)));
    }

    pub fn set_stereo<U, V>(&self, stereo: Stereo<U, V>)
    where
        U: SigT<Item = T> + Sync + Send + 'static,
        V: SigT<Item = T> + Sync + Send + 'static,
    {
        let ref_ = self.as_ref();
        ref_.left.set(stereo.left);
        ref_.right.set(stereo.right);
    }
}

impl StereoCellF32 {
    pub fn with_volume<V>(self, volume: V) -> Self
    where
        V: SigT<Item = f32> + Sync + Send + 'static,
    {
        let out = StereoPair::new_fn(|| cell(0.));
        let volume = sig_shared(volume);
        self.set_channel(|channel| out.get(channel).clone() * volume.clone());
        out
    }
}
