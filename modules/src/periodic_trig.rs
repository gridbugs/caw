use caw_builder_proc_macros::builder;
use caw_core::{sig_ops::sig_div, Buf, Sig, SigCtx, SigT};
use itertools::izip;

pub struct PeriodicTrig<P>
where
    P: SigT<Item = f32>,
{
    period_s: P,
    remaining_s: f32,
    buf: Vec<bool>,
}

impl<P> SigT for PeriodicTrig<P>
where
    P: SigT<Item = f32>,
{
    type Item = bool;

    fn sample(&mut self, ctx: &SigCtx) -> impl caw_core::Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, false);
        let sample_period_s = 1.0 / ctx.sample_rate_hz;
        let period_s = self.period_s.sample(ctx);
        for (out, period_s) in izip! {
            self.buf.iter_mut(),
            period_s.iter(),
        } {
            self.remaining_s =
                (self.remaining_s - sample_period_s).min(period_s);
            *out = if self.remaining_s < 0.0 {
                self.remaining_s = period_s;
                true
            } else {
                false
            };
        }
        &self.buf
    }
}

impl<P> PeriodicTrig<P>
where
    P: SigT<Item = f32>,
{
    fn new_s(period_s: P) -> Sig<Self> {
        Sig(Self {
            period_s,
            remaining_s: 0.0,
            buf: Vec::new(),
        })
    }
}

impl<P> PeriodicTrig<Sig<sig_div::OpScalarSig<f32, P>>>
where
    P: SigT<Item = f32>,
{
    fn new_hz(freq_hz: P) -> Sig<Self> {
        let period_s = 1.0 / Sig(freq_hz);
        Sig(Self {
            period_s,
            remaining_s: 0.0,
            buf: Vec::new(),
        })
    }
}

builder! {
    #[constructor = "periodic_trig_s"]
    #[constructor_doc = "A periodic trigger defined in terms of its period in seconds"]
    #[build_fn = "PeriodicTrig::new_s"]
    #[build_ty = "Sig<PeriodicTrig<P>>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilderS {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "P"]
        period_s: _,
    }
}

builder! {
    #[constructor = "periodic_trig_hz"]
    #[constructor_doc = "A periodic trigger defined in terms of its frequency in Hertz"]
    #[build_fn = "PeriodicTrig::new_hz"]
    #[build_ty = "Sig<PeriodicTrig<Sig<sig_div::OpScalarSig<f32, P>>>>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilderHz {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "P"]
        freq_hz: _,
    }
}
