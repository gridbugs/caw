use crate::signal::{Gate, Sf64, Signal, Trigger};
use std::cell::Cell;

pub struct AdsrLinear01 {
    pub key_down: Gate,
    pub key_press: Trigger,
    pub attack_s: Sf64,
    pub decay_s: Sf64,
    pub sustain_01: Sf64,
    pub release_s: Sf64,
}

impl AdsrLinear01 {
    pub fn signal(self) -> Sf64 {
        let current = Cell::new(0.0);
        let crossed_threshold = Cell::new(false);
        Signal::from_fn(move |ctx| {
            if self.key_press.sample(ctx) {
                crossed_threshold.set(false);
            }
            let mut current_value = current.get();
            if self.key_down.sample(ctx) {
                if crossed_threshold.get() {
                    // decay and sustain
                    current_value = (current_value
                        - (1.0
                            / (self.decay_s.sample(ctx)
                                * ctx.sample_rate_hz)))
                        .max(self.sustain_01.sample(ctx));
                } else {
                    // attack
                    current_value = (current_value
                        + (1.0
                            / (self.attack_s.sample(ctx)
                                * ctx.sample_rate_hz)))
                        .min(1.0);
                    if current_value == 1.0 {
                        crossed_threshold.set(true);
                    }
                }
            } else {
                // release
                crossed_threshold.set(false);
                current_value = (current_value
                    - (1.0
                        / (self.release_s.sample(ctx)
                            * ctx.sample_rate_hz)))
                    .max(0.0);
            }
            current.set(current_value);
            current_value
        })
    }
}
