use caw_builder_proc_macros::builder;
use caw_core::{Buf, Sig, SigCtx, SigT};
use itertools::izip;

pub struct AdsrLinear01<KD, KP, C, P, A, D, S, R>
where
    KD: SigT<Item = bool>,
    KP: SigT<Item = bool>,
    C: SigT<Item = bool>,
    P: SigT<Item = f32>,
    A: SigT<Item = f32>,
    D: SigT<Item = f32>,
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    key_down_gate: KD,
    key_press_trig: KP,
    clear_trig: C,
    pause_s: P,
    attack_s: A,
    decay_s: D,
    sustain_01: S,
    release_s: R,
    remaining_pause: f32,
    current: f32,
    crossed_threshold: bool,
    buf: Vec<f32>,
}

impl<KD, KP, C, P, A, D, S, R> SigT for AdsrLinear01<KD, KP, C, P, A, D, S, R>
where
    KD: SigT<Item = bool>,
    KP: SigT<Item = bool>,
    C: SigT<Item = bool>,
    P: SigT<Item = f32>,
    A: SigT<Item = f32>,
    D: SigT<Item = f32>,
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.clear();
        let key_down_gate = self.key_down_gate.sample(ctx);
        let key_press_trig = self.key_press_trig.sample(ctx);
        let clear_trig = self.clear_trig.sample(ctx);
        let pause_s = self.pause_s.sample(ctx);
        let attack_s = self.attack_s.sample(ctx);
        let decay_s = self.decay_s.sample(ctx);
        let sustain_01 = self.sustain_01.sample(ctx);
        let release_s = self.release_s.sample(ctx);

        for (
            key_down_gate,
            key_press_trig,
            clear_trig,
            pause_s,
            attack_s,
            decay_s,
            sustain_01,
            release_s,
        ) in izip! {
            key_down_gate.iter(),
            key_press_trig.iter(),
            clear_trig.iter(),
            pause_s.iter(),
            attack_s.iter(),
            decay_s.iter(),
            sustain_01.iter(),
            release_s.iter(),
        } {
            if clear_trig {
                self.remaining_pause = 1.0;
                self.current = 0.0;
                self.crossed_threshold = false;
            }
            if key_press_trig {
                self.crossed_threshold = false;
            }
            if key_down_gate {
                self.remaining_pause = (self.remaining_pause
                    - (1.0 / (pause_s * ctx.sample_rate_hz)))
                    .max(0.0);
                if self.remaining_pause == 0.0 {
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
                }
            } else {
                // release
                self.remaining_pause = 1.0;
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

impl<KD, KP, C, P, A, D, S, R> AdsrLinear01<KD, KP, C, P, A, D, S, R>
where
    KD: SigT<Item = bool>,
    KP: SigT<Item = bool>,
    C: SigT<Item = bool>,
    P: SigT<Item = f32>,
    A: SigT<Item = f32>,
    D: SigT<Item = f32>,
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    fn new(
        key_down_gate: KD,
        key_press_trig: KP,
        clear_trig: C,
        pause_s: P,
        attack_s: A,
        decay_s: D,
        sustain_01: S,
        release_s: R,
    ) -> Sig<Self> {
        Sig(Self {
            key_down_gate,
            key_press_trig,
            clear_trig,
            pause_s,
            attack_s,
            decay_s,
            sustain_01,
            release_s,
            remaining_pause: 0.0,
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
    #[build_ty = "Sig<AdsrLinear01<KD, KP, C, P, A, D, S, R>>"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "KD"]
        key_down_gate: _,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "KP"]
        #[default = false]
        key_press_trig: bool,
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "C"]
        #[default = false]
        clear_trig: bool,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "P"]
        #[default = 0.0]
        pause_s: f32,
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

impl<KD, KP, C, P, A, D, S, R> Props<KD, KP, C, P, A, D, S, R>
where
    KD: SigT<Item = bool>,
    KP: SigT<Item = bool>,
    C: SigT<Item = bool>,
    P: SigT<Item = f32>,
    A: SigT<Item = f32>,
    D: SigT<Item = f32>,
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    pub fn p<P_>(self, p: P_) -> Props<KD, KP, C, P_, A, D, S, R>
    where
        P_: SigT<Item = f32>,
    {
        self.pause_s(p)
    }

    pub fn a<A_>(self, a: A_) -> Props<KD, KP, C, P, A_, D, S, R>
    where
        A_: SigT<Item = f32>,
    {
        self.attack_s(a)
    }

    pub fn d<D_>(self, d: D_) -> Props<KD, KP, C, P, A, D_, S, R>
    where
        D_: SigT<Item = f32>,
    {
        self.decay_s(d)
    }

    pub fn s<S_>(self, s: S_) -> Props<KD, KP, C, P, A, D, S_, R>
    where
        S_: SigT<Item = f32>,
    {
        self.sustain_01(s)
    }

    pub fn r<R_>(self, r: R_) -> Props<KD, KP, C, P, A, D, S, R_>
    where
        R_: SigT<Item = f32>,
    {
        self.release_s(r)
    }
}
