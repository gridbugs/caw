use crate::signal::{Filter, Sf64, SignalCtx};
use std::{cell::RefCell, f64::consts::PI};

// This is the Oberheim Variation Model implementation of the Moog Ladder low pass filter. It's
// based on a reference implementation by Will Pirkle which can be found here:
// https://github.com/ddiakopoulos/MoogLadders/blob/master/src/OberheimVariationModel.h

struct VAOnePole {
    alpha: f64,
    beta: f64,
    gamma: f64,
    delta: f64,
    epsilon: f64,
    a0: f64,
    feedback: f64,
    z1: f64,
}

impl Default for VAOnePole {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            beta: 0.0,
            gamma: 1.0,
            delta: 0.0,
            epsilon: 0.0,
            a0: 1.0,
            feedback: 0.0,
            z1: 0.0,
        }
    }
}

impl VAOnePole {
    fn feedback_output(&self) -> f64 {
        self.beta * (self.z1 + (self.feedback * self.delta))
    }

    fn tick(&mut self, mut s: f64) -> f64 {
        s = (s * self.gamma)
            + self.feedback
            + (self.epsilon * self.feedback_output());
        let vn = ((self.a0 * s) - self.z1) * self.alpha;
        let out = vn + self.z1;
        self.z1 = vn + out;
        out
    }
}

#[derive(Default)]
struct OberheimVariationMoogState {
    lpf1: VAOnePole,
    lpf2: VAOnePole,
    lpf3: VAOnePole,
    lpf4: VAOnePole,
    k: f64,
    gamma: f64,
    alpha0: f64,
    q: f64,
    saturation: f64,
    cutoff_hz: f64,
    resonance: f64,
    sample_rate_hz: f64,
}

impl OberheimVariationMoogState {
    fn new() -> Self {
        let mut s = Self::default();
        s.sample_rate_hz = 44100.0;
        s.saturation = 1.0;
        s.q = 3.0;
        s.set_cutoff_hz(1000.0);
        s.set_resonance(0.0);
        s
    }

    fn set_resonance(&mut self, resonance: f64) {
        self.resonance = resonance;
        self.k = resonance * 4.0;
    }

    fn set_cutoff_hz(&mut self, cutoff_hz: f64) {
        self.cutoff_hz = cutoff_hz;
        // prewarp for BZT
        let wd = 2.0 * PI * cutoff_hz;
        let t = 1.0 / self.sample_rate_hz;
        let wa = (2.0 / t) * (wd * t / 2.0).tan();
        let g = wa * t / 2.0;
        let feedforward_coeff = g / (1.0 + g);
        self.lpf1.alpha = feedforward_coeff;
        self.lpf2.alpha = feedforward_coeff;
        self.lpf3.alpha = feedforward_coeff;
        self.lpf4.alpha = feedforward_coeff;
        self.lpf1.beta =
            (feedforward_coeff * feedforward_coeff * feedforward_coeff)
                / (1.0 + g);
        self.lpf2.beta = (feedforward_coeff * feedforward_coeff) / (1.0 + g);
        self.lpf3.beta = feedforward_coeff / (1.0 + g);
        self.lpf4.beta = 1.0 / (1.0 + g);
        self.gamma = feedforward_coeff
            * feedforward_coeff
            * feedforward_coeff
            * feedforward_coeff;
        self.alpha0 = 1.0 / (1.0 + (self.k * self.gamma));
    }

    fn process_sample(&mut self, mut sample: f64) -> f64 {
        let sigma = self.lpf1.feedback_output()
            + self.lpf2.feedback_output()
            + self.lpf3.feedback_output()
            + self.lpf4.feedback_output();
        sample *= 1.0 + self.k;
        // calculate input to first filter
        let u = (sample - (self.k * sigma)) * self.alpha0;
        let u = (self.saturation * u).tanh();
        let stage1 = self.lpf1.tick(u);
        let stage2 = self.lpf2.tick(stage1);
        let stage3 = self.lpf3.tick(stage2);

        self.lpf4.tick(stage3)
    }
}

pub struct LowPassMoogLadder {
    state: RefCell<OberheimVariationMoogState>,
    cutoff_hz: Sf64,
    resonance: Sf64,
}

impl LowPassMoogLadder {
    pub fn new(cutoff_hz: impl Into<Sf64>, resonance: impl Into<Sf64>) -> Self {
        Self {
            state: RefCell::new(OberheimVariationMoogState::new()),
            cutoff_hz: cutoff_hz.into(),
            resonance: resonance.into(),
        }
    }
}

const MAX_CUTOFF_FREQ_HZ: f64 = 20_000.0;

impl Filter for LowPassMoogLadder {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        let cutoff_hz =
            self.cutoff_hz.sample(ctx).clamp(0.0, MAX_CUTOFF_FREQ_HZ);
        let resonance = self.resonance.sample(ctx);
        let mut state = self.state.borrow_mut();
        if state.sample_rate_hz == ctx.sample_rate_hz {
            if cutoff_hz != state.cutoff_hz {
                state.set_cutoff_hz(cutoff_hz);
            }
        } else {
            state.sample_rate_hz = ctx.sample_rate_hz;
            state.set_cutoff_hz(cutoff_hz);
        }
        if resonance != state.resonance {
            state.set_resonance(resonance);
        }
        state.process_sample(input)
    }
}
