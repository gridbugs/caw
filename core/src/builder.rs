pub mod env {
    use crate::{
        envelope::AdsrLinear01,
        signal::{const_, Gate, Sf64, Trigger},
    };

    pub struct AdsrLinear01Builder {
        key_down: Gate,
        key_press: Option<Trigger>,
        attack_s: Option<Sf64>,
        decay_s: Option<Sf64>,
        sustain_01: Option<Sf64>,
        release_s: Option<Sf64>,
    }

    impl AdsrLinear01Builder {
        pub fn new(gate: impl Into<Gate>) -> Self {
            Self {
                key_down: gate.into(),
                key_press: None,
                attack_s: None,
                decay_s: None,
                sustain_01: None,
                release_s: None,
            }
        }

        pub fn key_press(mut self, key_press: impl Into<Trigger>) -> Self {
            self.key_press = Some(key_press.into());
            self
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
                key_press: self
                    .key_press
                    .unwrap_or_else(|| self.key_down.to_trigger_rising_edge()),
                key_down: self.key_down,
                attack_s: self.attack_s.unwrap_or_else(|| const_(0.0)),
                decay_s: self.decay_s.unwrap_or_else(|| const_(0.0)),
                sustain_01: self.sustain_01.unwrap_or_else(|| const_(1.0)),
                release_s: self.release_s.unwrap_or_else(|| const_(0.0)),
            }
            .signal()
        }
    }

    pub fn adsr_linear_01(key_down: impl Into<Gate>) -> AdsrLinear01Builder {
        AdsrLinear01Builder::new(key_down)
    }
}

pub mod oscillator {
    use crate::{
        oscillator::{Oscillator, Waveform},
        signal::{const_, sfreq_hz, sfreq_s, Sf64, Sfreq, Signal, Trigger},
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
}

pub mod gate {
    use crate::{
        clock::{PeriodicGate, PeriodicTrigger},
        signal::{const_, sfreq_hz, sfreq_s, Gate, Sf64, Sfreq, Trigger},
    };

    pub struct PeriodicGateBuilder {
        freq: Sfreq,
        duty_01: Option<Sf64>,
        offset_01: Option<Sf64>,
    }

    impl PeriodicGateBuilder {
        pub fn new(freq: impl Into<Sfreq>) -> Self {
            Self {
                freq: freq.into(),
                duty_01: None,
                offset_01: None,
            }
        }

        pub fn duty_01(mut self, duty_01: impl Into<Sf64>) -> Self {
            self.duty_01 = Some(duty_01.into());
            self
        }

        pub fn offset_01(mut self, offset_01: impl Into<Sf64>) -> Self {
            self.offset_01 = Some(offset_01.into());
            self
        }

        pub fn build(self) -> Gate {
            PeriodicGate {
                freq: self.freq,
                duty_01: self.duty_01.unwrap_or_else(|| const_(0.5)),
                offset_01: self.offset_01.unwrap_or_else(|| const_(0.0)),
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

    pub struct PeriodicTriggerBuilder(PeriodicTrigger);

    impl PeriodicTriggerBuilder {
        pub fn new(freq: impl Into<Sfreq>) -> Self {
            Self(PeriodicTrigger::new(freq))
        }

        pub fn build(self) -> Trigger {
            self.0.trigger()
        }
    }

    pub fn periodic_trigger(freq: impl Into<Sfreq>) -> PeriodicTriggerBuilder {
        PeriodicTriggerBuilder::new(freq)
    }

    pub fn periodic_trigger_hz(freq_hz: impl Into<Sf64>) -> PeriodicTriggerBuilder {
        PeriodicTriggerBuilder::new(sfreq_hz(freq_hz))
    }

    pub fn periodic_trigger_s(freq_s: impl Into<Sf64>) -> PeriodicTriggerBuilder {
        PeriodicTriggerBuilder::new(sfreq_s(freq_s))
    }
}

pub mod filter {
    use crate::{
        filters::*,
        signal::{const_, Sf64, Sfreq, Trigger},
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

    pub struct CompressBuilder {
        threshold: Option<Sf64>,
        ratio: Option<Sf64>,
        scale: Option<Sf64>,
    }

    impl CompressBuilder {
        pub fn new() -> Self {
            Self {
                threshold: None,
                ratio: None,
                scale: None,
            }
        }

        pub fn threshold(mut self, threshold: impl Into<Sf64>) -> Self {
            self.threshold = Some(threshold.into());
            self
        }

        pub fn ratio(mut self, ratio: impl Into<Sf64>) -> Self {
            self.ratio = Some(ratio.into());
            self
        }

        pub fn scale(mut self, scale: impl Into<Sf64>) -> Self {
            self.scale = Some(scale.into());
            self
        }

        pub fn build(self) -> Compress {
            Compress {
                threshold: self.threshold.unwrap_or_else(|| const_(1.0)),
                ratio: self.ratio.unwrap_or_else(|| const_(0.0)),
                scale: self.scale.unwrap_or_else(|| const_(1.0)),
            }
        }
    }

    pub struct DelayBuilder {
        time_s: Option<Sf64>,
    }

    impl DelayBuilder {
        pub fn new() -> Self {
            Self { time_s: None }
        }

        pub fn time_s(mut self, time_s: impl Into<Sf64>) -> Self {
            self.time_s = Some(time_s.into());
            self
        }

        pub fn build(self) -> Delay {
            Delay::new(self.time_s.unwrap_or_else(|| const_(0.0)))
        }
    }

    pub struct EchoBuilder {
        time_s: Option<Sf64>,
        scale: Option<Sf64>,
    }

    impl EchoBuilder {
        pub fn new() -> Self {
            Self {
                time_s: None,
                scale: None,
            }
        }

        pub fn time_s(mut self, time_s: impl Into<Sf64>) -> Self {
            self.time_s = Some(time_s.into());
            self
        }

        pub fn scale(mut self, scale: impl Into<Sf64>) -> Self {
            self.scale = Some(scale.into());
            self
        }

        pub fn build(self) -> Echo {
            Echo::new(
                self.time_s.unwrap_or_else(|| const_(1.0)),
                self.scale.unwrap_or_else(|| const_(0.5)),
            )
        }
    }

    pub struct SampleAndHoldBuilder(SampleAndHold);

    impl SampleAndHoldBuilder {
        pub fn build(self) -> SampleAndHold {
            self.0
        }
    }

    pub struct QuantizeBuilder(Quantize);

    impl QuantizeBuilder {
        pub fn build(self) -> Quantize {
            self.0
        }
    }

    pub struct DownSampleBuilder(DownSample);

    impl DownSampleBuilder {
        pub fn build(self) -> DownSample {
            self.0
        }
    }

    pub struct QuantizeToScaleBuilder(QuantizeToScale);

    impl QuantizeToScaleBuilder {
        pub fn build(self) -> QuantizeToScale {
            self.0
        }
    }

    pub struct ReverbBuilder {
        damping: Option<Sf64>,
        room_size: Option<Sf64>,
    }

    impl ReverbBuilder {
        pub fn new() -> Self {
            Self {
                room_size: None,
                damping: None,
            }
        }

        pub fn room_size(mut self, room_size: impl Into<Sf64>) -> Self {
            self.room_size = Some(room_size.into());
            self
        }

        pub fn damping(mut self, damping: impl Into<Sf64>) -> Self {
            self.damping = Some(damping.into());
            self
        }

        pub fn build(self) -> Reverb {
            Reverb::new(
                self.room_size
                    .unwrap_or_else(|| const_(Reverb::DEFAULT_ROOM_SIZE)),
                self.damping
                    .unwrap_or_else(|| const_(Reverb::DEFAULT_DAMPING)),
            )
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

    pub fn compress() -> CompressBuilder {
        CompressBuilder::new()
    }

    pub fn delay() -> DelayBuilder {
        DelayBuilder::new()
    }

    pub fn delay_s(time_s: impl Into<Sf64>) -> Delay {
        delay().time_s(time_s).build()
    }

    pub fn echo() -> EchoBuilder {
        EchoBuilder::new()
    }

    pub fn sample_and_hold(trigger: Trigger) -> SampleAndHoldBuilder {
        SampleAndHoldBuilder(SampleAndHold::new(trigger))
    }

    pub fn quantize(resolution: impl Into<Sf64>) -> QuantizeBuilder {
        QuantizeBuilder(Quantize::new(resolution))
    }

    pub fn down_sample(scale: impl Into<Sf64>) -> DownSampleBuilder {
        DownSampleBuilder(DownSample::new(scale))
    }

    pub fn quantize_to_scale(notes: Vec<Sfreq>) -> QuantizeToScaleBuilder {
        QuantizeToScaleBuilder(QuantizeToScale::new(notes))
    }

    pub fn reverb() -> ReverbBuilder {
        ReverbBuilder::new()
    }
}

pub mod loopers {
    use crate::{
        loopers::*,
        signal::{const_, Gate, Su8, Trigger},
    };

    pub struct ClockedTriggerLooperBuilder {
        clock: Option<Trigger>,
        add: Option<Gate>,
        remove: Option<Gate>,
        length: Option<usize>,
    }

    impl ClockedTriggerLooperBuilder {
        pub fn new() -> Self {
            Self {
                clock: None,
                add: None,
                remove: None,
                length: None,
            }
        }

        pub fn clock(mut self, clock: impl Into<Trigger>) -> Self {
            self.clock = Some(clock.into());
            self
        }

        pub fn add(mut self, add: impl Into<Gate>) -> Self {
            self.add = Some(add.into());
            self
        }

        pub fn remove(mut self, remove: impl Into<Gate>) -> Self {
            self.remove = Some(remove.into());
            self
        }

        pub fn length(mut self, length: usize) -> Self {
            self.length = Some(length.into());
            self
        }

        pub fn build(self) -> Trigger {
            ClockedTriggerLooper {
                clock: self.clock.unwrap_or_else(|| Trigger::never()),
                add: self.add.unwrap_or_else(|| Gate::never()),
                remove: self.remove.unwrap_or_else(|| Gate::never()),
                length: self.length.unwrap_or(8),
            }
            .trigger()
        }
    }

    pub struct ClockedMidiNoteMonophonicLooperBuilder {
        clock: Option<Trigger>,
        input_gate: Option<Gate>,
        input_midi_index: Option<Su8>,
        clear: Option<Gate>,
        length: Option<usize>,
    }

    impl ClockedMidiNoteMonophonicLooperBuilder {
        pub fn new() -> Self {
            Self {
                clock: None,
                input_gate: None,
                input_midi_index: None,
                clear: None,
                length: None,
            }
        }

        pub fn clock(mut self, clock: impl Into<Trigger>) -> Self {
            self.clock = Some(clock.into());
            self
        }

        pub fn input_gate(mut self, input_gate: impl Into<Gate>) -> Self {
            self.input_gate = Some(input_gate.into());
            self
        }

        pub fn input_midi_index(mut self, input_midi_index: impl Into<Su8>) -> Self {
            self.input_midi_index = Some(input_midi_index.into());
            self
        }

        pub fn clear(mut self, clear: impl Into<Gate>) -> Self {
            self.clear = Some(clear.into());
            self
        }

        pub fn length(mut self, length: usize) -> Self {
            self.length = Some(length.into());
            self
        }

        pub fn build(self) -> (Gate, Su8) {
            ClockedMidiNoteMonophonicLooper {
                clock: self.clock.unwrap_or_else(|| Trigger::never()),
                input_gate: self.input_gate.unwrap_or_else(|| Gate::never()),
                input_midi_index: self.input_midi_index.unwrap_or_else(|| const_(0)),
                clear: self.clear.unwrap_or_else(|| Gate::never()),
                length: self.length.unwrap_or(8),
            }
            .signal()
        }
    }

    pub fn clocked_trigger_looper() -> ClockedTriggerLooperBuilder {
        ClockedTriggerLooperBuilder::new()
    }

    pub fn clocked_midi_note_monophonic_looper() -> ClockedMidiNoteMonophonicLooperBuilder {
        ClockedMidiNoteMonophonicLooperBuilder::new()
    }
}

pub mod sampler {
    pub use crate::sampler::{Sample, Sampler};
    use crate::signal::{Sf64, Trigger};

    pub struct SamplerBuilder<'a> {
        sample: &'a Sample,
        trigger: Option<Trigger>,
    }

    impl<'a> SamplerBuilder<'a> {
        pub fn new(sample: &'a Sample) -> Self {
            Self {
                sample,
                trigger: None,
            }
        }

        pub fn trigger(mut self, trigger: impl Into<Trigger>) -> Self {
            self.trigger = Some(trigger.into());
            self
        }

        pub fn build(self) -> Sf64 {
            Sampler::new(self.sample, self.trigger.unwrap_or_else(Trigger::never)).signal()
        }
    }

    pub fn sampler(sample: &Sample) -> SamplerBuilder {
        SamplerBuilder::new(sample)
    }
}

pub mod patches {
    use crate::{
        patches,
        signal::{const_, sfreq_hz, Sf64, Sfreq, Trigger},
    };

    pub struct SupersawBuilder {
        resolution: Option<usize>,
        freq: Sfreq,
        ratio: Option<Sf64>,
        reset_trigger: Option<Trigger>,
        reset_offset_01: Option<Sf64>,
    }

    impl SupersawBuilder {
        pub fn new(freq: impl Into<Sfreq>) -> Self {
            Self {
                resolution: None,
                freq: freq.into(),
                ratio: None,
                reset_trigger: None,
                reset_offset_01: None,
            }
        }

        pub fn resolution(mut self, resolution: impl Into<usize>) -> Self {
            self.resolution = Some(resolution.into());
            self
        }

        pub fn ratio(mut self, ratio: impl Into<Sf64>) -> Self {
            self.ratio = Some(ratio.into());
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
            patches::supersaw(
                self.resolution.unwrap_or(1),
                self.freq,
                self.ratio.unwrap_or_else(|| const_(0.01)),
                self.reset_trigger.unwrap_or_else(|| Trigger::never()),
                self.reset_offset_01.unwrap_or_else(|| const_(0.0)),
            )
        }
    }

    pub fn supersaw(freq: impl Into<Sfreq>) -> SupersawBuilder {
        SupersawBuilder::new(freq)
    }

    pub fn supersaw_hz(freq_hz: impl Into<Sf64>) -> SupersawBuilder {
        supersaw(sfreq_hz(freq_hz))
    }

    pub struct PulsePwmBuilder {
        osc_freq: Sfreq,
        pwm_freq: Option<Sfreq>,
        offset_01: Option<Sf64>,
        scale_01: Option<Sf64>,
        reset_trigger: Option<Trigger>,
        reset_offset_01: Option<Sf64>,
    }

    impl PulsePwmBuilder {
        pub fn new(osc_freq: impl Into<Sfreq>) -> Self {
            Self {
                osc_freq: osc_freq.into(),
                pwm_freq: None,
                offset_01: None,
                scale_01: None,
                reset_trigger: None,
                reset_offset_01: None,
            }
        }

        pub fn pwm_freq(mut self, pwm_freq: impl Into<Sfreq>) -> Self {
            self.pwm_freq = Some(pwm_freq.into());
            self
        }

        pub fn offset_01(mut self, offset_01: impl Into<Sf64>) -> Self {
            self.offset_01 = Some(offset_01.into());
            self
        }

        pub fn scale_01(mut self, scale_01: impl Into<Sf64>) -> Self {
            self.scale_01 = Some(scale_01.into());
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
            patches::pulse_pwm(
                self.osc_freq,
                self.pwm_freq.unwrap_or_else(|| sfreq_hz(const_(1.0))),
                self.offset_01.unwrap_or_else(|| const_(0.5)),
                self.scale_01.unwrap_or_else(|| const_(0.5)),
                self.reset_trigger.unwrap_or_else(|| Trigger::never()),
                self.reset_offset_01.unwrap_or_else(|| const_(0.0)),
            )
        }
    }

    pub fn pulse_pwm(freq: impl Into<Sfreq>) -> PulsePwmBuilder {
        PulsePwmBuilder::new(freq)
    }

    pub fn pulse_pwm_hz(freq_hz: impl Into<Sf64>) -> PulsePwmBuilder {
        pulse_pwm(sfreq_hz(freq_hz))
    }
}
