use crate::{
    builder,
    oscillator::Waveform,
    signal::{Gate, Sf64, Sfreq},
};

pub struct PeriodicGate {
    pub freq: Sfreq,
    pub duty_01: Sf64,
}

impl PeriodicGate {
    pub fn new(freq: impl Into<Sfreq>, duty_01: impl Into<Sf64>) -> Self {
        Self {
            freq: freq.into(),
            duty_01: duty_01.into(),
        }
    }

    pub fn gate(self) -> Gate {
        builder::signal::oscillator(Waveform::Pulse, self.freq)
            .pulse_width_01(self.duty_01)
            .build()
            .map(|x| x < 0.0)
            .to_gate()
    }
}
