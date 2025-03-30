use crate::low_level::{
    moog_ladder::MoogLadderState,
    moog_ladder_huovilainen::State as HuovilainenState,
    moog_ladder_oberheim::State as OberheimState,
};
use caw_builder_proc_macros::builder;
use caw_core::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

builder! {
    #[constructor = "low_pass_moog_ladder_oberheim"]
    #[constructor_doc = "A low pass filter with adjustable resonance based on the Oberheim model of the Moog Ladder"]
    #[generic_setter_type_name = "X"]
    pub struct PropsOberheim {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "C"]
        cutoff_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.0]
        resonance: f32,
    }
}

builder! {
    #[constructor = "low_pass_moog_ladder_huovilainen"]
    #[constructor_doc = "A low pass filter with adjustable resonance based on the Huovilainen model of the Moog Ladder. Compared to the Oberheim model it is more accurate but more expensive."]
    #[generic_setter_type_name = "X"]
    pub struct PropsHuovilainen {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "C"]
        cutoff_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.0]
        resonance: f32,
    }
}

impl<C: SigT<Item = f32>, R: SigT<Item = f32>> PropsOberheim<C, R> {
    pub fn q<X>(self, resonance: X) -> PropsOberheim<C, X>
    where
        X: SigT<Item = f32>,
    {
        self.resonance(resonance)
    }
}

impl<C: SigT<Item = f32>, R: SigT<Item = f32>> PropsHuovilainen<C, R> {
    pub fn q<X>(self, resonance: X) -> PropsHuovilainen<C, X>
    where
        X: SigT<Item = f32>,
    {
        self.resonance(resonance)
    }
}

impl<C, R> Filter for PropsOberheim<C, R>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = LowPassMoogLadder<S, C, R, OberheimState>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        LowPassMoogLadder {
            cutoff_hz: self.cutoff_hz,
            resonance: self.resonance,
            sig,
            state: OberheimState::new(),
            buf: Vec::new(),
        }
    }
}

impl<C, R> Filter for PropsHuovilainen<C, R>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = LowPassMoogLadder<S, C, R, HuovilainenState>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        LowPassMoogLadder {
            cutoff_hz: self.cutoff_hz,
            resonance: self.resonance,
            sig,
            state: HuovilainenState::new(),
            buf: Vec::new(),
        }
    }
}

pub struct LowPassMoogLadder<S, C, R, M>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
    M: MoogLadderState,
{
    cutoff_hz: C,
    resonance: R,
    sig: S,
    state: M,
    buf: Vec<f32>,
}

impl<S, C, R, M> SigT for LowPassMoogLadder<S, C, R, M>
where
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
    M: MoogLadderState,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        let sig = self.sig.sample(ctx);
        let cutoff_hz = self.cutoff_hz.sample(ctx);
        let resonance = self.resonance.sample(ctx);
        for (out, sample, cutoff_hz, resonance) in izip! {
            self.buf.iter_mut(),
            sig.iter(),
            cutoff_hz.iter(),
            resonance.iter(),
        } {
            self.state.update_params(
                ctx.sample_rate_hz as f64,
                cutoff_hz as f64,
                resonance as f64,
            );
            *out = self.state.process_sample(sample as f64) as f32;
        }
        &self.buf
    }
}
