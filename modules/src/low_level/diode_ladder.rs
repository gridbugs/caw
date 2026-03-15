// A diode ladder low-pass filter based on the diodeladder filter from loopmaster
// (https://github.com/loopmaster-xyz/engine).

use std::f64;

const TWO_PI: f64 = f64::consts::PI * 2.;

const DIODELADDER_Q_COMP: f64 = 2.5;
const DIODELADDER_K_COMP: f64 = 1.0;

#[derive(PartialEq, Default, Clone, Copy)]
struct Params {
    cutoff_hz: f64,
    resonance_01: f64,
    hpf_01: f64,
}

/// Projections of inputs. Only need to be recomputed when the inputs change.
#[derive(Default)]
struct Control {
    a_: f64,
    ah: f64,
    bh: f64,
    a: f64,
    a2: f64,
    b: f64,
    b2: f64,
    c: f64,
    g: f64,
    g0: f64,
    ainv: f64,
    k_res: f64,
}

impl Control {
    fn from_params(
        Params {
            cutoff_hz: cutoff,
            resonance_01: q,
            hpf_01: k,
        }: Params,
        sample_rate_hz: f64,
    ) -> Self {
        let nyquist = sample_rate_hz * 0.5;
        let cut_norm = cutoff / nyquist;
        let qq = q * q;
        let ah = (k * (TWO_PI / 2.0) - 2.0) / (k * (TWO_PI / 2.0) + 2.0);
        let bh = 2.0 / (k * (TWO_PI / 2.0) + 2.0);
        let k_res = 20.0 * q;
        let a_ = 1.0 + 0.5 * k_res;
        let comp = 1.0 + DIODELADDER_Q_COMP * qq + DIODELADDER_K_COMP * (k * q);
        let cut_comp = 5.0f64.max(((cut_norm / comp) * nyquist).min(nyquist));
        let a = (TWO_PI / 2.0) * (cut_comp / nyquist);
        let a = 2.0 * (0.5 * a).sin() / (0.5 * a).cos();
        let ainv = 1.0 / a;
        let a2 = a * a;
        let b = 2.0 * a + 1.0;
        let b2 = b * b;
        let a2x2 = 2.0 * a2 * a2;
        let c = 1.0 / (a2x2 - 4.0 * a2 * b2 + b2 * b2);
        let g0 = a2x2 * c;
        let g = g0 * bh;
        Self {
            a_,
            ah,
            bh,
            a,
            a2,
            b,
            b2,
            c,
            g,
            g0,
            ainv,
            k_res,
        }
    }
}

#[derive(Default)]
struct State {
    z0: f64,
    z1: f64,
    z2: f64,
    z3: f64,
    z4: f64,
}

impl State {
    /// Compute the output of the filter given the current sample, updating the state. Note that
    /// saturation is not part of `Control` because its value is only used directly when computing
    /// the next sample rather than projected into the fields of `Control`.
    fn compute(
        &mut self,
        sample: f64,
        Control {
            a_,
            ah,
            bh,
            a,
            a2,
            b,
            b2,
            c,
            g,
            g0,
            ainv,
            k_res,
        }: &Control,
        saturation: f64,
    ) -> f64 {
        let saturation = saturation.max(0.1);
        let s0 = (a2 * a * self.z0
            + a2 * b * self.z1
            + self.z2 * (b2 - 2.0 * a2) * a
            + self.z3 * (b2 - 3.0 * a2) * b)
            * c;
        let s = bh * s0 - self.z4;
        let y5 = (g * sample + s) / (1.0 + g * k_res);
        let x_in = sample - k_res * y5;
        let y0 = x_in / ((1.0 / saturation) + x_in.abs());
        let y5 = g * y0 + s;
        let y4 = g0 * y0 + s0;
        let y3 = (b * y4 - self.z3) * ainv;
        let y2 = (b * y3 - a * y4 - self.z2) * ainv;
        let y1 = (b * y2 - a * y3 - self.z1) * ainv;
        self.z0 = self.z0 + 4.0 * a * (y0 - y1 + y2);
        self.z1 = self.z1 + 2.0 * a * (y1 - 2.0 * y2 + y3);
        self.z2 = self.z2 + 2.0 * a * (y2 - 2.0 * y3 + y4);
        self.z3 = self.z3 + 2.0 * a * (y3 - 2.0 * y4);
        self.z4 = bh * y4 + ah * y5;
        a_ * y4
    }
}

#[derive(Default)]
pub struct DiodeLadder {
    params: Params,
    control: Control,
    state: State,
}

impl DiodeLadder {
    pub fn process_sample(
        &mut self,
        sample: f64,
        cutoff_hz: f64,
        resonance_01: f64,
        hpf_01: f64,
        saturation: f64,
        sample_rate_hz: f64,
    ) -> f64 {
        let params = Params {
            cutoff_hz,
            resonance_01,
            hpf_01,
        };
        if self.params != params {
            self.params = params;
            self.control = Control::from_params(params, sample_rate_hz);
        }
        self.state.compute(sample, &self.control, saturation)
    }
}
