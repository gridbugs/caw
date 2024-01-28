use crate::{
    builder,
    oscillator::Waveform,
    signal::{Gate, Sf64, Sfreq, Trigger},
};

pub struct PeriodicGate {
    pub freq: Sfreq,
    pub duty_01: Sf64,
    pub offset_01: Sf64,
}

impl PeriodicGate {
    pub fn new(
        freq: impl Into<Sfreq>,
        duty_01: impl Into<Sf64>,
        offset_01: impl Into<Sf64>,
    ) -> Self {
        Self {
            freq: freq.into(),
            duty_01: duty_01.into(),
            offset_01: offset_01.into(),
        }
    }

    pub fn gate(self) -> Gate {
        builder::oscillator::oscillator(Waveform::Pulse, self.freq)
            .pulse_width_01(self.duty_01)
            .reset_offset_01(self.offset_01)
            .build()
            .map(|x| x < 0.0)
            .to_gate()
    }
}

pub struct PeriodicTrigger {
    pub freq: Sfreq,
}

impl PeriodicTrigger {
    pub fn new(freq: impl Into<Sfreq>) -> Self {
        Self { freq: freq.into() }
    }

    pub fn trigger(self) -> Trigger {
        builder::oscillator::oscillator(Waveform::Pulse, self.freq)
            .build()
            .map(|x| x < 0.0)
            .to_trigger_rising_edge()
    }
}
