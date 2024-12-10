use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Sig, SigCtx, SigT};
use itertools::izip;

pub struct PeriodicGate<P, D>
where
    P: SigT<Item = f32>,
    D: SigT<Item = f32>,
{
    period_s: P,
    duty_01: D,
    buf: Vec<bool>,
    remaining_s: f32,
}

impl<P, D> SigT for PeriodicGate<P, D>
where
    P: SigT<Item = f32>,
    D: SigT<Item = f32>,
{
    type Item = bool;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let sample_period_s = 1.0 / ctx.sample_rate_hz;
        self.buf.clear();
        for (period_s, duty_01) in izip! {
            self.period_s.sample(ctx).iter(),
            self.duty_01.sample(ctx).iter(),
        } {
            // If the period drops below the remaining time on the current tick, set the remaining
            // time on the current tick to the new period. This way if there is a very long period,
            // and the period is decreased to a smaller value, we don't need to wait the entire
            // long period.
            self.remaining_s =
                (self.remaining_s - sample_period_s).min(period_s);
            if self.remaining_s < 0.0 {
                self.remaining_s = period_s;
            }
            self.buf.push(self.remaining_s / period_s < duty_01);
        }
        &self.buf
    }
}

impl<P, D> PeriodicGate<P, D>
where
    P: SigT<Item = f32>,
    D: SigT<Item = f32>,
{
    fn new(period_s: P, duty_01: D) -> Sig<Self> {
        Sig(Self {
            period_s,
            duty_01,
            buf: Vec::new(),
            remaining_s: 0.0,
        })
    }
}

builder! {
    #[constructor = "periodic_gate_s"]
    #[constructor_doc = "A periodic gate defined in terms of its period in seconds with a duty cycle defined as a ratio of the period"]
    #[build_fn = "PeriodicGate::new"]
    #[build_ty = "Sig<PeriodicGate<P, D>>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "P"]
        period_s: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "D"]
        #[default = 0.5]
        duty_01: f32,
    }
}
