//! Huovilainen's model of the Moog Ladder low-pass filter.
//! Reference implementation here:
//! https://github.com/ddiakopoulos/MoogLadders/blob/master/src/HuovilainenModel.h

use std::f64::consts::PI;

#[derive(Default)]
pub struct State {
    stage: [f64; 4],
    stage_tanh: [f64; 3],
    delay: [f64; 6],
    thermal: f64,
    tune: f64,
    acr: f64,
    res_quad: f64,
    cutoff_hz: f64,
    resonance: f64,
    sample_rate_hz: f64,
}

impl State {
    pub fn new() -> Self {
        let mut s = Self {
            sample_rate_hz: 44100.0,
            thermal: 0.000025,
            ..Default::default()
        };
        s.set_cutoff_hz(1000.0);
        s.set_resonance(0.1);
        s
    }

    fn set_resonance(&mut self, resonance: f64) {
        self.resonance = resonance;
        self.res_quad = 4.0 * self.resonance * self.acr;
    }

    fn set_cutoff_hz(&mut self, cutoff_hz: f64) {
        self.cutoff_hz = cutoff_hz;
        let fc = cutoff_hz / self.sample_rate_hz;
        let f = fc * 0.5;
        let fc2 = fc * fc;
        let fc3 = fc2 * fc;
        let fcr = 1.8730 * fc3 + 0.4955 * fc2 - 0.6490 * fc + 0.9988;
        self.acr = -3.9364 * fc2 + 1.8409 * fc + 0.9968;
        self.tune = (1.0 - (-((2.0 * PI) * f * fcr)).exp()) / self.thermal;
        self.res_quad = 4.0 * self.resonance * self.acr;
    }

    pub fn update_params(
        &mut self,
        sample_rate_hz: f64,
        cutoff_hz: f64,
        resonance: f64,
    ) {
        let cutoff_hz = cutoff_hz.max(0.0);
        let resonance = resonance.min(0.99);
        if sample_rate_hz == self.sample_rate_hz {
            if cutoff_hz != self.cutoff_hz {
                self.set_cutoff_hz(cutoff_hz);
            }
        } else {
            self.sample_rate_hz = sample_rate_hz;
            self.set_cutoff_hz(cutoff_hz);
        }
        if resonance != self.resonance {
            self.set_resonance(resonance);
        }
    }

    pub fn process_sample(&mut self, sample: f64) -> f64 {
        for _ in 0..2 {
            let mut input = sample - self.res_quad * self.delay[5];
            self.delay[0] += self.tune
                * ((input * self.thermal).tanh() - self.stage_tanh[0]);
            self.stage[0] = self.delay[0];
            for k in 1..4 {
                input = self.stage[k - 1];
                self.stage_tanh[k - 1] = (input * self.thermal).tanh();
                self.stage[k] = self.delay[k]
                    + self.tune
                        * (self.stage_tanh[k - 1]
                            - (if k != 3 {
                                self.stage_tanh[k]
                            } else {
                                (self.delay[k] * self.thermal).tanh()
                            }));
                self.delay[k] = self.stage[k];
            }
            // 0.5 sample delay for phase compensation
            self.delay[5] = (self.stage[3] + self.delay[4]) * 0.5;
            self.delay[4] = self.stage[3];
        }
        self.delay[5]
    }

    pub fn resonance(&self) -> f64 {
        self.resonance
    }
}
