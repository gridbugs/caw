use caw_builder_proc_macros::builder;
use caw_core_next::{frame_sig_ops::sig_div, FrameSig, FrameSigT, SigCtx};

pub struct PeriodicTrig<P>
where
    P: FrameSigT<Item = f32>,
{
    period_s: P,
    remaining_s: f32,
}

impl<P> FrameSigT for PeriodicTrig<P>
where
    P: FrameSigT<Item = f32>,
{
    type Item = bool;

    fn frame_sample(&mut self, ctx: &SigCtx) -> Self::Item {
        let sample_period_s = ctx.num_samples as f32 / ctx.sample_rate_hz;
        let period_s = self.period_s.frame_sample(ctx);
        // If the period drops below the remaining time on the current tick, set the remaining
        // time on the current tick to the new period. This way if there is a very long period,
        // and the period is decreased to a smaller value, we don't need to wait the entire
        // long period.
        self.remaining_s = (self.remaining_s - sample_period_s).min(period_s);
        if self.remaining_s < 0.0 {
            self.remaining_s = period_s;
            true
        } else {
            false
        }
    }
}

impl<P> PeriodicTrig<P>
where
    P: FrameSigT<Item = f32>,
{
    fn new_s(period_s: P) -> FrameSig<Self> {
        FrameSig(Self {
            period_s,
            remaining_s: 0.0,
        })
    }
}

impl<P> PeriodicTrig<FrameSig<sig_div::Op<f32, P>>>
where
    P: FrameSigT<Item = f32>,
{
    fn new_hz(freq_hz: P) -> FrameSig<Self> {
        let period_s: FrameSig<sig_div::Op<f32, P>> = 1.0 / FrameSig(freq_hz);
        FrameSig(Self {
            period_s,
            remaining_s: 0.0,
        })
    }
}

builder! {
    #[constructor = "periodic_trig_s"]
    #[constructor_doc = "A periodic trigger defined in terms of its period in seconds"]
    #[build_fn = "PeriodicTrig::new_s"]
    #[build_ty = "FrameSig<PeriodicTrig<P>>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilderS {
        #[generic_with_constraint = "FrameSigT<Item = f32>"]
        #[generic_name = "P"]
        period_s: _,
    }
}

builder! {
    #[constructor = "periodic_trig_hz"]
    #[constructor_doc = "A periodic trigger defined in terms of its frequency in Hertz"]
    #[build_fn = "PeriodicTrig::new_hz"]
    #[build_ty = "FrameSig<PeriodicTrig<FrameSig<sig_div::Op<f32, P>>>>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilderHz {
        #[generic_with_constraint = "FrameSigT<Item = f32>"]
        #[generic_name = "P"]
        freq_hz: _,
    }
}
