use caw_builder_proc_macros::builder;
use caw_core_next::{sig_ops::sig_div, Buf, Sig, SigCtx, SigT};

pub struct PeriodicTrig<P>
where
    P: SigT<Item = f32>,
{
    period_s: P,
    buf: Vec<bool>,
    remaining_s: f32,
}

impl<P> SigT for PeriodicTrig<P>
where
    P: SigT<Item = f32>,
{
    type Item = bool;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let sample_period_s = 1.0 / ctx.sample_rate_hz;
        self.buf.clear();
        for period_s in self.period_s.sample(ctx).iter() {
            // If the period drops below the remaining time on the current tick, set the remaining
            // time on the current tick to the new period. This way if there is a very long period,
            // and the period is decreased to a smaller value, we don't need to wait the entire
            // long period.
            self.remaining_s =
                (self.remaining_s - sample_period_s).min(period_s);
            let trig_value = if self.remaining_s < 0.0 {
                self.remaining_s = period_s;
                true
            } else {
                false
            };
            self.buf.push(trig_value);
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
            buf: Vec::new(),
            remaining_s: 0.0,
        })
    }
}

impl<P> PeriodicTrig<Sig<sig_div::OpScalarSig<f32, P>>>
where
    P: SigT<Item = f32>,
{
    fn new_hz(freq_hz: P) -> Sig<Self> {
        let period_s: Sig<sig_div::OpScalarSig<f32, P>> = 1.0 / Sig(freq_hz);
        Sig(Self {
            period_s,
            buf: Vec::new(),
            remaining_s: 0.0,
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
