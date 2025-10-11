use std::{
    fmt::Display,
    ops::{Add, Mul},
};

use crate::{Buf, Sig, SigCtx, SigSampleIntoBufT, SigT, sig_ops::sig_add};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Left,
    Right,
}

impl Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Left => write!(f, "left"),
            Self::Right => write!(f, "right"),
        }
    }
}

impl Channel {
    // Apply a 90 degree offset to the right channel so a stereo sine wave would draw a circle with
    // stereo oscillographics.
    pub fn circle_phase_offset_01(&self) -> f32 {
        match self {
            Self::Left => 0.0,
            Self::Right => 0.25,
        }
    }

    pub fn is_left(&self) -> bool {
        *self == Self::Left
    }

    pub fn is_right(&self) -> bool {
        *self == Self::Right
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Stereo<L, R> {
    pub left: L,
    pub right: R,
}

pub type StereoPair<T> = Stereo<T, T>;

impl<L, R> Stereo<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }

    pub fn zip<OL, OR>(
        self,
        other: Stereo<OL, OR>,
    ) -> Stereo<(L, OL), (R, OR)> {
        Stereo {
            left: (self.left, other.left),
            right: (self.right, other.right),
        }
    }

    pub fn zip3<O1L, O1R, O2L, O2R>(
        self,
        other1: Stereo<O1L, O1R>,
        other2: Stereo<O2L, O2R>,
    ) -> Stereo<(L, O1L, O2L), (R, O1R, O2R)> {
        Stereo {
            left: (self.left, other1.left, other2.left),
            right: (self.right, other1.right, other2.right),
        }
    }

    pub fn zip_ref<'a, 'b, OL, OR>(
        &'a self,
        other: &'b Stereo<OL, OR>,
    ) -> Stereo<(&'a L, &'b OL), (&'a R, &'b OR)> {
        self.as_ref().zip(other.as_ref())
    }

    pub fn zip_ref3<'a, 'b, 'c, O1L, O1R, O2L, O2R>(
        &'a self,
        other1: &'b Stereo<O1L, O1R>,
        other2: &'c Stereo<O2L, O2R>,
    ) -> Stereo<(&'a L, &'b O1L, &'c O2L), (&'a R, &'b O1R, &'c O2R)> {
        self.as_ref().zip3(other1.as_ref(), other2.as_ref())
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

    pub fn as_ref(&self) -> Stereo<&L, &R> {
        Stereo::new(&self.left, &self.right)
    }

    pub fn as_mut(&mut self) -> Stereo<&mut L, &mut R> {
        Stereo::new(&mut self.left, &mut self.right)
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

impl<L, R> Stereo<L, R>
where
    L: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    pub fn sum(self) -> Sig<sig_add::OpSigSig<L, R>> {
        Sig(self.left) + Sig(self.right)
    }
}

impl<L, R> Stereo<L, R>
where
    L: SigSampleIntoBufT,
    R: SigSampleIntoBufT,
{
    pub fn sample_into_buf<'a>(
        &'a mut self,
        ctx: &'a SigCtx,
        buf: Stereo<&mut Vec<L::Item>, &mut Vec<R::Item>>,
    ) {
        self.left.sample_into_buf(ctx, buf.left);
        self.right.sample_into_buf(ctx, buf.right);
    }
}

impl<S> Stereo<S, S> {
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

    pub fn get(&self, channel: Channel) -> &S {
        match channel {
            Channel::Left => &self.left,
            Channel::Right => &self.right,
        }
    }

    pub fn get_mut(&mut self, channel: Channel) -> &mut S {
        match channel {
            Channel::Left => &mut self.left,
            Channel::Right => &mut self.right,
        }
    }

    pub fn with<F>(self, mut f: F)
    where
        F: FnMut(S),
    {
        f(self.left);
        f(self.right);
    }

    pub fn with_channel<F>(self, mut f: F)
    where
        F: FnMut(Channel, S),
    {
        f(Channel::Left, self.left);
        f(Channel::Right, self.right);
    }

    pub fn map_pair<O, F>(self, mut f: F) -> Stereo<O, O>
    where
        F: FnMut(S) -> O,
    {
        Stereo {
            left: f(self.left),
            right: f(self.right),
        }
    }

    pub fn map_ref_pair<O, F>(&self, mut f: F) -> Stereo<O, O>
    where
        F: FnMut(&S) -> O,
    {
        Stereo {
            left: f(&self.left),
            right: f(&self.right),
        }
    }

    pub fn map_pair_channel<O, F>(self, mut f: F) -> Stereo<O, O>
    where
        F: FnMut(S, Channel) -> O,
    {
        Stereo {
            left: f(self.left, Channel::Left),
            right: f(self.right, Channel::Right),
        }
    }

    pub fn map_ref_pair_channel<O, F>(&self, mut f: F) -> Stereo<O, O>
    where
        F: FnMut(&S, Channel) -> O,
    {
        Stereo {
            left: f(&self.left, Channel::Left),
            right: f(&self.right, Channel::Right),
        }
    }
}

impl<LL, LR, RL, RR> Add<Stereo<RL, RR>> for Stereo<LL, LR>
where
    LL: Add<RL>,
    LR: Add<RR>,
{
    type Output = Stereo<LL::Output, LR::Output>;
    fn add(self, rhs: Stereo<RL, RR>) -> Self::Output {
        Stereo::new(self.left + rhs.left, self.right + rhs.right)
    }
}

impl<L, R, RHS> Mul<RHS> for Stereo<L, R>
where
    RHS: Clone,
    L: Mul<RHS>,
    R: Mul<RHS>,
{
    type Output = Stereo<L::Output, R::Output>;
    fn mul(self, rhs: RHS) -> Self::Output {
        Stereo::new(self.left * rhs.clone(), self.right * rhs)
    }
}
