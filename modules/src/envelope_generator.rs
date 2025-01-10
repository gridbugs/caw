use caw_builder_proc_macros::builder;
use caw_core::{Buf, Sig, SigCtx, SigT};
use itertools::izip;

pub struct AdsrLinear01<KD, KP, A, D, S, R>
where
    KD: SigT<Item = bool>,
    KP: SigT<Item = bool>,
    A: SigT<Item = f32>,
    D: SigT<Item = f32>,
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    key_down_gate: KD,
    key_press_trig: KP,
    attack_s: A,
    decay_s: D,
    sustain_01: S,
    release_s: R,
    current: f32,
    crossed_threshold: bool,
    buf: Vec<f32>,
}

impl<KD, KP, A, D, S, R> SigT for AdsrLinear01<KD, KP, A, D, S, R>
where
    KD: SigT<Item = bool>,
    KP: SigT<Item = bool>,
    A: SigT<Item = f32>,
    D: SigT<Item = f32>,
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.clear();
        for (
            key_down_gate,
            key_press_trig,
            attack_s,
            decay_s,
            sustain_01,
            release_s,
        ) in izip! {
            self.key_down_gate.sample(ctx).iter(),
            self.key_press_trig.sample(ctx).iter(),
            self.attack_s.sample(ctx).iter(),
            self.decay_s.sample(ctx).iter(),
            self.sustain_01.sample(ctx).iter(),
            self.release_s.sample(ctx).iter(),
        } {
            if key_press_trig {
                self.crossed_threshold = false;
            }
            if key_down_gate {
                if self.crossed_threshold {
                    // decay and sustain
                    self.current = (self.current
                        - (1.0 / (decay_s * ctx.sample_rate_hz)))
                        .max(sustain_01);
                } else {
                    // attack
                    self.current = (self.current
                        + (1.0 / (attack_s * ctx.sample_rate_hz)))
                        .min(1.0);
                    if self.current == 1.0 {
                        self.crossed_threshold = true;
                    }
                }
            } else {
                // release
                self.crossed_threshold = false;
                self.current = (self.current
                    - (1.0 / (release_s * ctx.sample_rate_hz)))
                    .max(0.0);
            }
            self.buf.push(self.current)
        }
        &self.buf
    }
}

impl<KD, KP, A, D, S, R> AdsrLinear01<KD, KP, A, D, S, R>
where
    KD: SigT<Item = bool>,
    KP: SigT<Item = bool>,
    A: SigT<Item = f32>,
    D: SigT<Item = f32>,
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    fn new(
        key_down_gate: KD,
        key_press_trig: KP,
        attack_s: A,
        decay_s: D,
        sustain_01: S,
        release_s: R,
    ) -> Sig<Self> {
        Sig(Self {
            key_down_gate,
            key_press_trig,
            attack_s,
            decay_s,
            sustain_01,
            release_s,
            current: 0.0,
            crossed_threshold: false,
            buf: Vec::new(),
        })
    }
}

builder! {
    #[constructor = "adsr_linear_01"]
    #[constructor_doc = "An ADSR envelope generator where all the slopes are linear."]
    #[build_fn = "AdsrLinear01::new"]
    #[build_ty = "Sig<AdsrLinear01<KD, KP, A, D, S, R>>"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "KD"]
        key_down_gate: _,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "KP"]
        #[default = false]
        key_press_trig: bool,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "A"]
        #[default = 0.0]
        attack_s: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "D"]
        #[default = 0.0]
        decay_s: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "S"]
        #[default = 1.0]
        sustain_01: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 0.0]
        release_s: f32,
    }
}
