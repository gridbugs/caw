use caw_builder_proc_macros::builder;
use caw_core::{FrameSig, FrameSigT, SigCtx};

pub struct PeriodicGate<P, D>
where
    P: FrameSigT<Item = f32>,
    D: FrameSigT<Item = f32>,
{
    period_s: P,
    duty_01: D,
    remaining_s: f32,
}

impl<P, D> FrameSigT for PeriodicGate<P, D>
where
    P: FrameSigT<Item = f32>,
    D: FrameSigT<Item = f32>,
{
    type Item = bool;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        let period_s = self.period_s.frame_sample(ctx);
        let duty_01 = self.duty_01.frame_sample(ctx);
        // If the period drops below the remaining time on the current tick, set the remaining
        // time on the current tick to the new period. This way if there is a very long period,
        // and the period is decreased to a smaller value, we don't need to wait the entire
        // long period.
        self.remaining_s =
            (self.remaining_s - ctx.sample_period_s()).min(period_s);
        let ret = (self.remaining_s / period_s) < duty_01;
        if self.remaining_s < 0.0 {
            self.remaining_s = period_s;
        }
        ret
    }
}

impl<P, D> PeriodicGate<P, D>
where
    P: FrameSigT<Item = f32>,
    D: FrameSigT<Item = f32>,
{
    fn new(period_s: P, duty_01: D) -> FrameSig<Self> {
        FrameSig(Self {
            period_s,
            duty_01,
            remaining_s: 0.0,
        })
    }
}

builder! {
    #[constructor = "periodic_gate_s"]
    #[constructor_doc = "A periodic gate defined in terms of its period in seconds with a duty cycle defined as a ratio of the period"]
    #[build_fn = "PeriodicGate::new"]
    #[build_ty = "FrameSig<PeriodicGate<P, D>>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "FrameSigT<Item = f32>"]
        #[generic_name = "P"]
        period_s: _,
        #[generic_with_constraint = "FrameSigT<Item = f32>"]
        #[generic_name = "D"]
        #[default = 0.5]
        duty_01: f32,
    }
}
