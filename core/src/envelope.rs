use crate::signal::{Gate, Sf64, Signal};
use std::cell::Cell;

pub struct AdsrLinear01 {
    pub gate: Gate,
    pub attack_s: Sf64,
    pub decay_s: Sf64,
    pub sustain_01: Sf64,
    pub release_s: Sf64,
}

impl AdsrLinear01 {
    pub fn new(
        gate: impl Into<Gate>,
        attack_s: impl Into<Sf64>,
        decay_s: impl Into<Sf64>,
        sustain_01: impl Into<Sf64>,
        release_s: impl Into<Sf64>,
    ) -> Self {
        Self {
            gate: gate.into(),
            attack_s: attack_s.into(),
            decay_s: decay_s.into(),
            sustain_01: sustain_01.into(),
            release_s: release_s.into(),
        }
    }

    pub fn signal(self) -> Sf64 {
        let current = Cell::new(0.0);
        let crossed_threshold = Cell::new(false);
        Signal::from_fn(move |ctx| {
            let mut current_value = current.get();
            if self.gate.sample(ctx) {
                if crossed_threshold.get() {
                    // decay and sustain
                    current_value = (current_value
                        - (1.0 / (self.decay_s.sample(ctx) * ctx.sample_rate_hz as f64)))
                        .max(self.sustain_01.sample(ctx));
                } else {
                    // attack
                    current_value = (current_value
                        + (1.0 / (self.attack_s.sample(ctx) * ctx.sample_rate_hz as f64)))
                        .min(1.0);
                    if current_value == 1.0 {
                        crossed_threshold.set(true);
                    }
                }
            } else {
                // release
                crossed_threshold.set(false);
                current_value = (current_value
                    - (1.0 / (self.release_s.sample(ctx) * ctx.sample_rate_hz as f64)))
                    .max(0.0);
            }
            current.set(current_value);
            current_value
        })
    }
}
