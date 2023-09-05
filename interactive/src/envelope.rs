use crate::signal::{const_, Gate, Sf64, Signal};

pub struct AdsrLinear01 {
    pub gate: Gate,
    pub attack_s: Sf64,
    pub decay_s: Sf64,
    pub sustain_01: Sf64,
    pub release_s: Sf64,
}

pub struct AdsrLinear01Builder {
    gate: Gate,
    attack_s: Option<Sf64>,
    decay_s: Option<Sf64>,
    sustain_01: Option<Sf64>,
    release_s: Option<Sf64>,
}

impl AdsrLinear01Builder {
    pub fn new(gate: Gate) -> Self {
        Self {
            gate,
            attack_s: None,
            decay_s: None,
            sustain_01: None,
            release_s: None,
        }
    }

    pub fn attack_s(mut self, attack_s: impl Into<Sf64>) -> Self {
        self.attack_s = Some(attack_s.into());
        self
    }

    pub fn decay_s(mut self, decay_s: impl Into<Sf64>) -> Self {
        self.decay_s = Some(decay_s.into());
        self
    }

    pub fn sustain_01(mut self, sustain_01: impl Into<Sf64>) -> Self {
        self.sustain_01 = Some(sustain_01.into());
        self
    }

    pub fn release_s(mut self, release_s: impl Into<Sf64>) -> Self {
        self.release_s = Some(release_s.into());
        self
    }

    pub fn build(self) -> AdsrLinear01 {
        AdsrLinear01 {
            gate: self.gate,
            attack_s: self.attack_s.unwrap_or_else(|| const_(0.0)),
            decay_s: self.decay_s.unwrap_or_else(|| const_(0.0)),
            sustain_01: self.sustain_01.unwrap_or_else(|| const_(1.0)),
            release_s: self.release_s.unwrap_or_else(|| const_(0.0)),
        }
    }

    pub fn build_signal(self) -> Sf64 {
        self.build().signal()
    }
}

impl AdsrLinear01 {
    pub fn new(
        gate: Gate,
        attack_s: impl Into<Sf64>,
        decay_s: impl Into<Sf64>,
        sustain_01: impl Into<Sf64>,
        release_s: impl Into<Sf64>,
    ) -> Self {
        Self {
            gate,
            attack_s: attack_s.into(),
            decay_s: decay_s.into(),
            sustain_01: sustain_01.into(),
            release_s: release_s.into(),
        }
    }

    pub fn builder(gate: Gate) -> AdsrLinear01Builder {
        AdsrLinear01Builder::new(gate)
    }

    pub fn signal(mut self) -> Sf64 {
        let mut current_value = 0.0;
        let mut crossed_threshold = false;
        Signal::from_fn(move |ctx| {
            if self.gate.sample(ctx) {
                if crossed_threshold {
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
                        crossed_threshold = true;
                    }
                }
            } else {
                // release
                crossed_threshold = false;
                current_value = (current_value
                    - (1.0 / (self.release_s.sample(ctx) * ctx.sample_rate_hz as f64)))
                    .max(0.0);
            }
            current_value
        })
    }
}
