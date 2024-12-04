use crate::{Buf, SigCtx, SigT};

#[derive(Debug, Clone, Copy)]
pub enum Channel {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Stereo<L, R> {
    pub left: L,
    pub right: R,
}

impl<L, R> Stereo<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }

    pub fn map_ref<'a, OL, FL, OR, FR>(
        &'a self,
        fl: FL,
        fr: FR,
    ) -> Stereo<OL, OR>
    where
        FL: FnOnce(&'a L) -> OL,
        FR: FnOnce(&'a R) -> OR,
    {
        Stereo {
            left: fl(&self.left),
            right: fr(&self.right),
        }
    }
}

impl<L, R> Stereo<L, R>
where
    L: SigT,
    R: SigT,
{
    pub fn sample<'a>(
        &'a mut self,
        ctx: &'a SigCtx,
    ) -> Stereo<impl 'a + Buf<L::Item>, impl 'a + Buf<R::Item>> {
        Stereo {
            left: self.left.sample(ctx),
            right: self.right.sample(ctx),
        }
    }
}

impl<S> Stereo<S, S>
where
    S: SigT,
{
    pub fn new_fn<F>(mut f: F) -> Self
    where
        F: FnMut() -> S,
    {
        Self {
            left: f(),
            right: f(),
        }
    }

    pub fn new_fn_channel<F>(mut f: F) -> Self
    where
        F: FnMut(Channel) -> S,
    {
        Self {
            left: f(Channel::Left),
            right: f(Channel::Right),
        }
    }
}
