use crate::{
    oscillator::{Oscillator, Waveform},
    signal::{const_, Gate, Sf64, Sfreq},
};

pub struct PeriodicGate {
    pub frequency: Sfreq,
    pub duty_01: Sf64,
}

pub struct PeriodicGateBuilder {
    frequency: Sfreq,
    duty_01: Option<Sf64>,
}

impl PeriodicGateBuilder {
    pub fn new(frequency: impl Into<Sfreq>) -> Self {
        Self {
            frequency: frequency.into(),
            duty_01: None,
        }
    }

    pub fn duty_01(mut self, duty_01: impl Into<Sf64>) -> Self {
        self.duty_01 = Some(duty_01.into());
        self
    }

    pub fn build(self) -> PeriodicGate {
        PeriodicGate {
            frequency: self.frequency,
            duty_01: self.duty_01.unwrap_or_else(|| const_(0.5)),
        }
    }

    pub fn build_gate(self) -> Gate {
        self.build().gate()
    }
}

impl PeriodicGate {
    pub fn new(frequency: impl Into<Sfreq>, duty_01: impl Into<Sf64>) -> Self {
        Self {
            frequency: frequency.into(),
            duty_01: duty_01.into(),
        }
    }

    pub fn builder(frequency: impl Into<Sfreq>) -> PeriodicGateBuilder {
        PeriodicGateBuilder::new(frequency)
    }

    pub fn gate(self) -> Gate {
        Oscillator::builder(Waveform::Pulse, self.frequency)
            .pulse_width_01(self.duty_01)
            .build_signal()
            .map(|x| x < 0.0)
            .to_gate()
    }
}
