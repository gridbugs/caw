pub mod signal {
    use crate::{
        envelope::AdsrLinear01,
        oscillator::{Oscillator, Waveform},
        signal::{const_, sfreq_hz, sfreq_s, Gate, Sf64, Sfreq, Signal, Trigger},
    };

    pub struct OscillatorBuilder {
        waveform: Signal<Waveform>,
        freq: Sfreq,
        pulse_width_01: Option<Sf64>,
        reset_trigger: Option<Trigger>,
        reset_offset_01: Option<Sf64>,
    }

    impl OscillatorBuilder {
        pub fn new(waveform: impl Into<Signal<Waveform>>, freq: impl Into<Sfreq>) -> Self {
            Self {
                waveform: waveform.into(),
                freq: freq.into(),
                pulse_width_01: None,
                reset_trigger: None,
                reset_offset_01: None,
            }
        }

        pub fn pulse_width_01(mut self, pulse_width_01: impl Into<Sf64>) -> Self {
            self.pulse_width_01 = Some(pulse_width_01.into());
            self
        }

        pub fn reset_trigger(mut self, reset_trigger: impl Into<Trigger>) -> Self {
            self.reset_trigger = Some(reset_trigger.into());
            self
        }

        pub fn reset_offset_01(mut self, reset_offset_01: impl Into<Sf64>) -> Self {
            self.reset_offset_01 = Some(reset_offset_01.into());
            self
        }

        pub fn build(self) -> Sf64 {
            Oscillator {
                waveform: self.waveform,
                freq: self.freq,
                pulse_width_01: self.pulse_width_01.unwrap_or_else(|| const_(0.5)),
                reset_trigger: self.reset_trigger.unwrap_or_else(|| Trigger::never()),
                reset_offset_01: self.reset_offset_01.unwrap_or_else(|| const_(0.0)),
            }
            .signal()
        }
    }

    pub struct AdsrLinear01Builder {
        gate: Gate,
        attack_s: Option<Sf64>,
        decay_s: Option<Sf64>,
        sustain_01: Option<Sf64>,
        release_s: Option<Sf64>,
    }

    impl AdsrLinear01Builder {
        pub fn new(gate: impl Into<Gate>) -> Self {
            Self {
                gate: gate.into(),
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

        pub fn build(self) -> Sf64 {
            AdsrLinear01 {
                gate: self.gate,
                attack_s: self.attack_s.unwrap_or_else(|| const_(0.0)),
                decay_s: self.decay_s.unwrap_or_else(|| const_(0.0)),
                sustain_01: self.sustain_01.unwrap_or_else(|| const_(1.0)),
                release_s: self.release_s.unwrap_or_else(|| const_(0.0)),
            }
            .signal()
        }
    }

    pub fn oscillator(
        waveform: impl Into<Signal<Waveform>>,
        freq: impl Into<Sfreq>,
    ) -> OscillatorBuilder {
        OscillatorBuilder::new(waveform, freq)
    }

    pub fn oscillator_hz(
        waveform: impl Into<Signal<Waveform>>,
        freq_hz: impl Into<Sf64>,
    ) -> OscillatorBuilder {
        OscillatorBuilder::new(waveform, sfreq_hz(freq_hz))
    }

    pub fn oscillator_s(
        waveform: impl Into<Signal<Waveform>>,
        freq_s: impl Into<Sf64>,
    ) -> OscillatorBuilder {
        OscillatorBuilder::new(waveform, sfreq_s(freq_s))
    }

    pub fn adsr_linear_01(gate: impl Into<Gate>) -> AdsrLinear01Builder {
        AdsrLinear01Builder::new(gate)
    }
}

pub mod gate {
    use crate::{
        clock::PeriodicGate,
        signal::{const_, sfreq_hz, sfreq_s, Gate, Sf64, Sfreq},
    };

    pub struct PeriodicGateBuilder {
        freq: Sfreq,
        duty_01: Option<Sf64>,
    }

    impl PeriodicGateBuilder {
        pub fn new(freq: impl Into<Sfreq>) -> Self {
            Self {
                freq: freq.into(),
                duty_01: None,
            }
        }

        pub fn duty_01(mut self, duty_01: impl Into<Sf64>) -> Self {
            self.duty_01 = Some(duty_01.into());
            self
        }

        pub fn build(self) -> Gate {
            PeriodicGate {
                freq: self.freq,
                duty_01: self.duty_01.unwrap_or_else(|| const_(0.5)),
            }
            .gate()
        }
    }

    pub fn periodic_gate(freq: impl Into<Sfreq>) -> PeriodicGateBuilder {
        PeriodicGateBuilder::new(freq)
    }

    pub fn periodic_gate_hz(freq_hz: impl Into<Sf64>) -> PeriodicGateBuilder {
        PeriodicGateBuilder::new(sfreq_hz(freq_hz))
    }

    pub fn periodic_gate_s(freq_s: impl Into<Sf64>) -> PeriodicGateBuilder {
        PeriodicGateBuilder::new(sfreq_s(freq_s))
    }
}

pub mod filter {
    use crate::{
        filters::*,
        signal::{const_, Sf64},
    };

    /// Included for consistency with other filters even though `LowPassButterworth` doesn't
    /// specifically benefit from the builder pattern.
    pub struct LowPassButterworthBuilder(LowPassButterworth);

    impl LowPassButterworthBuilder {
        pub fn build(self) -> LowPassButterworth {
            self.0
        }
    }

    /// Included for consistency with other filters even though `HighPassButterworth` doesn't
    /// specifically benefit from the builder pattern.
    pub struct HighPassButterworthBuilder(HighPassButterworth);

    impl HighPassButterworthBuilder {
        pub fn build(self) -> HighPassButterworth {
            self.0
        }
    }

    pub struct LowPassChebyshevBuilder {
        cutoff_hz: Sf64,
        resonance: Option<Sf64>,
    }

    impl LowPassChebyshevBuilder {
        pub fn new(cutoff_hz: impl Into<Sf64>) -> Self {
            Self {
                cutoff_hz: cutoff_hz.into(),
                resonance: None,
            }
        }

        pub fn resonance(mut self, resonance: impl Into<Sf64>) -> Self {
            self.resonance = Some(resonance.into());
            self
        }

        pub fn build(self) -> LowPassChebyshev {
            LowPassChebyshev::new(
                self.cutoff_hz,
                self.resonance.unwrap_or_else(|| const_(0.0)),
            )
        }
    }

    pub struct HighPassChebyshevBuilder {
        cutoff_hz: Sf64,
        resonance: Option<Sf64>,
    }

    impl HighPassChebyshevBuilder {
        pub fn new(cutoff_hz: impl Into<Sf64>) -> Self {
            Self {
                cutoff_hz: cutoff_hz.into(),
                resonance: None,
            }
        }

        pub fn resonance(mut self, resonance: impl Into<Sf64>) -> Self {
            self.resonance = Some(resonance.into());
            self
        }

        pub fn build(self) -> HighPassChebyshev {
            HighPassChebyshev::new(
                self.cutoff_hz,
                self.resonance.unwrap_or_else(|| const_(0.0)),
            )
        }
    }

    pub struct LowPassMoogLadderBuilder {
        cutoff_hz: Sf64,
        resonance: Option<Sf64>,
    }

    impl LowPassMoogLadderBuilder {
        pub fn new(cutoff_hz: impl Into<Sf64>) -> Self {
            Self {
                cutoff_hz: cutoff_hz.into(),
                resonance: None,
            }
        }

        pub fn resonance(mut self, resonance: impl Into<Sf64>) -> Self {
            self.resonance = Some(resonance.into());
            self
        }

        pub fn build(self) -> LowPassMoogLadder {
            LowPassMoogLadder::new(
                self.cutoff_hz,
                self.resonance.unwrap_or_else(|| const_(0.0)),
            )
        }
    }

    pub struct SaturateBuilder {
        scale: Option<Sf64>,
        max: Option<Sf64>,
        min: Option<Sf64>,
    }

    impl SaturateBuilder {
        pub fn new() -> Self {
            Self {
                scale: None,
                max: None,
                min: None,
            }
        }

        pub fn scale(mut self, scale: impl Into<Sf64>) -> Self {
            self.scale = Some(scale.into());
            self
        }

        pub fn min(mut self, min: impl Into<Sf64>) -> Self {
            self.min = Some(min.into());
            self
        }

        pub fn max(mut self, max: impl Into<Sf64>) -> Self {
            self.max = Some(max.into());
            self
        }

        pub fn threshold(mut self, threshold: impl Into<Sf64>) -> Self {
            let threshold = threshold.into();
            self.max = Some(threshold.clone());
            self.min = Some(threshold * -1.0);
            self
        }

        pub fn build(self) -> Saturate {
            Saturate {
                scale: self.scale.unwrap_or_else(|| const_(1.0)),
                min: self.min.unwrap_or_else(|| const_(-1.0)),
                max: self.max.unwrap_or_else(|| const_(1.0)),
            }
        }
    }

    pub fn low_pass_butterworth(cutoff_hz: impl Into<Sf64>) -> LowPassButterworthBuilder {
        LowPassButterworthBuilder(LowPassButterworth::new(cutoff_hz))
    }

    pub fn high_pass_butterworth(cutoff_hz: impl Into<Sf64>) -> HighPassButterworthBuilder {
        HighPassButterworthBuilder(HighPassButterworth::new(cutoff_hz))
    }

    pub fn low_pass_chebyshev(cutoff_hz: impl Into<Sf64>) -> LowPassChebyshevBuilder {
        LowPassChebyshevBuilder::new(cutoff_hz)
    }

    pub fn high_pass_chebyshev(cutoff_hz: impl Into<Sf64>) -> HighPassChebyshevBuilder {
        HighPassChebyshevBuilder::new(cutoff_hz)
    }

    pub fn low_pass_moog_ladder(cutoff_hz: impl Into<Sf64>) -> LowPassMoogLadderBuilder {
        LowPassMoogLadderBuilder::new(cutoff_hz)
    }

    pub fn saturate() -> SaturateBuilder {
        SaturateBuilder::new()
    }
}
