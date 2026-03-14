use crate::low_level::diode_ladder::DiodeLadder;
use caw_builder_proc_macros::builder;
use caw_core::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

builder! {
    #[constructor = "low_pass_diode_ladder"]
    #[constructor_doc = "A diode ladder filter"]
    #[generic_setter_type_name = "X"]
    pub struct PropsDiodeLadder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "C"]
        cutoff_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.0]
        resonance: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "H"]
        #[default = 0.0]
        high_pass: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "S"]
        #[default = 1.0]
        saturation: f32,
    }
}

impl<C, R, H, S> PropsDiodeLadder<C, R, H, S>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
    H: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    pub fn q<X>(self, resonance: X) -> PropsDiodeLadder<C, X, H, S>
    where
        X: SigT<Item = f32>,
    {
        self.resonance(resonance)
    }
}

impl<C, R, H, S> Filter for PropsDiodeLadder<C, R, H, S>
where
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
    H: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<X>
        = LowPassDiodeLadder<X, C, R, H, S>
    where
        X: SigT<Item = Self::ItemIn>;

    fn into_sig<X>(self, sig: X) -> Self::Out<X>
    where
        X: SigT<Item = Self::ItemIn>,
    {
        LowPassDiodeLadder {
            sig,
            props: self,
            buf: Vec::new(),
            diode_ladder: DiodeLadder::default(),
        }
    }
}

pub struct LowPassDiodeLadder<X, C, R, H, S>
where
    X: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
    H: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    sig: X,
    props: PropsDiodeLadder<C, R, H, S>,
    buf: Vec<f32>,
    diode_ladder: DiodeLadder,
}

impl<X, C, R, H, S> SigT for LowPassDiodeLadder<X, C, R, H, S>
where
    X: SigT<Item = f32>,
    C: SigT<Item = f32>,
    R: SigT<Item = f32>,
    H: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        let sig = self.sig.sample(ctx);
        let cutoff_hz = self.props.cutoff_hz.sample(ctx);
        let resonance = self.props.resonance.sample(ctx);
        let high_pass = self.props.high_pass.sample(ctx);
        let saturation = self.props.saturation.sample(ctx);
        for (out, sample, cutoff_hz, resonance, high_pass, saturation) in izip! {
            self.buf.iter_mut(),
            sig.iter(),
            cutoff_hz.iter(),
            resonance.iter(),
            high_pass.iter(),
            saturation.iter(),
        } {
            *out = self.diode_ladder.process_sample(
                sample as f64,
                cutoff_hz as f64,
                resonance as f64,
                high_pass as f64,
                saturation as f64,
                ctx.sample_rate_hz as f64,
            ) as f32;
        }
        &self.buf
    }
}
