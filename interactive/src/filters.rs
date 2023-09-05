use crate::signal::{const_, Filter, Sf64, SignalCtx};

mod biquad_filter {
    // This is based on the filter designs at:
    // https://exstrom.com/journal/sigproc/dsigproc.html
    // This module will take parameter names from the reference implementation for easier
    // correspondence between the two implementations, but the public API to this module will
    // change the names for consistency.

    #[derive(Default)]
    struct BufferEntry {
        a: f64,
        d1: f64,
        d2: f64,
        w0: f64,
        w1: f64,
        w2: f64,
    }

    pub struct Buffer {
        entries: Vec<BufferEntry>,
    }

    impl Buffer {
        pub fn new(filter_order_half: usize) -> Self {
            let mut entries = Vec::new();
            for _ in 0..filter_order_half {
                entries.push(Default::default());
            }
            Self { entries }
        }

        fn apply_low_pass(&mut self, mut sample: f64) -> f64 {
            for entry in self.entries.iter_mut() {
                entry.w0 = (entry.d1 * entry.w1) + (entry.d2 * entry.w2) + sample;
                sample = entry.a * (entry.w0 + (2.0 * entry.w1) + entry.w2);
                entry.w2 = entry.w1;
                entry.w1 = entry.w0;
            }
            sample
        }

        fn apply_high_pass(&mut self, mut sample: f64) -> f64 {
            for entry in self.entries.iter_mut() {
                entry.w0 = (entry.d1 * entry.w1) + (entry.d2 * entry.w2) + sample;
                sample = entry.a * (entry.w0 - (2.0 * entry.w1) + entry.w2);
                entry.w2 = entry.w1;
                entry.w1 = entry.w0;
            }
            sample
        }
    }

    pub trait PassTrait {
        fn apply(buffer: &mut Buffer, sample: f64) -> f64;
    }
    struct LowPass;
    struct HighPass;
    impl PassTrait for LowPass {
        fn apply(buffer: &mut Buffer, sample: f64) -> f64 {
            buffer.apply_low_pass(sample)
        }
    }
    impl PassTrait for HighPass {
        fn apply(buffer: &mut Buffer, sample: f64) -> f64 {
            buffer.apply_high_pass(sample)
        }
    }

    pub mod butterworth {
        use super::*;
        use crate::signal::*;

        trait UpdateBufferTrait {
            fn update_entries(buffer: &mut Buffer, half_power_frequency_hz: f64);
        }

        pub struct State {
            pub half_power_frequency_hz: Sf64,
            pub buffer: Buffer,
        }

        impl State {
            fn run<U: UpdateBufferTrait, P: PassTrait>(
                &mut self,
                sample: f64,
                ctx: &SignalCtx,
            ) -> f64 {
                if self.buffer.entries.is_empty() {
                    return sample;
                }
                let half_power_frequency_hz = self.half_power_frequency_hz.sample(ctx);
                let half_power_frequency_sample_rate_ratio =
                    half_power_frequency_hz / ctx.sample_rate_hz;
                U::update_entries(&mut self.buffer, half_power_frequency_sample_rate_ratio);
                P::apply(&mut self.buffer, sample)
            }
        }

        pub mod low_pass {
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(
                    buffer: &mut Buffer,
                    half_power_frequency_sample_rate_ratio: f64,
                ) {
                    let a = (PI * half_power_frequency_sample_rate_ratio).tan();
                    let a2 = a * a;
                    let n = buffer.entries.len() as f64;
                    for (i, entry) in buffer.entries.iter_mut().enumerate() {
                        let r = ((PI * ((2.0 * i as f64) + 1.0)) / (4.0 * n)).sin();
                        let s = a2 + (2.0 * a * r) + 1.0;
                        entry.a = a2 / s;
                        entry.d1 = (2.0 * (1.0 - a2)) / s;
                        entry.d2 = -(a2 - (2.0 * a * r) + 1.0) / s;
                    }
                }
            }

            pub fn run(state: &mut State, sample: f64, ctx: &SignalCtx) -> f64 {
                state.run::<UpdateBuffer, LowPass>(sample, ctx)
            }
        }

        pub mod high_pass {
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(
                    buffer: &mut Buffer,
                    half_power_frequency_sample_rate_ratio: f64,
                ) {
                    let a = (PI * half_power_frequency_sample_rate_ratio).tan();
                    let a2 = a * a;
                    let n = buffer.entries.len() as f64;
                    for (i, entry) in buffer.entries.iter_mut().enumerate() {
                        let r = ((PI * ((2.0 * i as f64) + 1.0)) / (4.0 * n)).sin();
                        let s = a2 + (2.0 * a * r) + 1.0;
                        entry.a = 1.0 / s;
                        entry.d1 = (2.0 * (1.0 - a2)) / s;
                        entry.d2 = -(a2 - (2.0 * a * r) + 1.0) / s;
                    }
                }
            }

            pub fn run(state: &mut State, sample: f64, ctx: &SignalCtx) -> f64 {
                state.run::<UpdateBuffer, HighPass>(sample, ctx)
            }
        }
    }

    pub mod chebyshev {
        use super::*;
        use crate::signal::*;

        pub const EPSILON_MIN: f64 = 0.01;

        trait UpdateBufferTrait {
            fn update_entries(buffer: &mut Buffer, cutoff_hz: f64, epsilon: f64);
        }

        pub struct State {
            pub cutoff_hz: Sf64,
            pub epsilon: Sf64,
            pub buffer: Buffer,
        }

        impl State {
            fn run<U: UpdateBufferTrait, P: PassTrait>(
                &mut self,
                sample: f64,
                ctx: &SignalCtx,
            ) -> f64 {
                if self.buffer.entries.is_empty() {
                    return sample;
                }
                let cutoff_hz = self.cutoff_hz.sample(ctx);
                let cutoff_sample_rate_ratio = cutoff_hz / ctx.sample_rate_hz;
                let epsilon = self.epsilon.sample(ctx).max(EPSILON_MIN);
                U::update_entries(&mut self.buffer, cutoff_sample_rate_ratio, epsilon);
                let output_scaled = P::apply(&mut self.buffer, sample);
                let scale_factor = (1.0 - (-epsilon).exp()) / 2.0;
                output_scaled / scale_factor
            }
        }

        pub mod low_pass {
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(
                    buffer: &mut Buffer,
                    cutoff_sample_rate_ratio: f64,
                    epsilon: f64,
                ) {
                    let a = (PI * cutoff_sample_rate_ratio).tan();
                    let a2 = a * a;
                    let u = ((1.0 + (1.0 + (epsilon * epsilon)).sqrt()) / epsilon).ln();
                    let n = (buffer.entries.len() * 2) as f64;
                    let su = (u / n).sinh();
                    let cu = (u / n).cosh();
                    for (i, entry) in buffer.entries.iter_mut().enumerate() {
                        let theta = (PI * ((2.0 * i as f64) + 1.0)) / (2.0 * n);
                        let b = theta.sin() * su;
                        let c = theta.cos() * cu;
                        let c = (b * b) + (c * c);
                        let s = (a2 * c) + (2.0 * a * b) + 1.0;
                        entry.a = a2 / (4.0 * s);
                        entry.d1 = (2.0 * (1.0 - (a2 * c))) / s;
                        entry.d2 = -((a2 * c) - (2.0 * a * b) + 1.0) / s;
                    }
                }
            }

            pub fn run(state: &mut State, sample: f64, ctx: &SignalCtx) -> f64 {
                state.run::<UpdateBuffer, LowPass>(sample, ctx)
            }
        }

        pub mod high_pass {
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(
                    buffer: &mut Buffer,
                    cutoff_sample_rate_ratio: f64,
                    epsilon: f64,
                ) {
                    let a = (PI * cutoff_sample_rate_ratio).tan();
                    let a2 = a * a;
                    let u = ((1.0 + (1.0 + (epsilon * epsilon)).sqrt()) / epsilon).ln();
                    let n = (buffer.entries.len() * 2) as f64;
                    let su = (u / n).sinh();
                    let cu = (u / n).cosh();
                    for (i, entry) in buffer.entries.iter_mut().enumerate() {
                        let theta = (PI * ((2.0 * i as f64) + 1.0)) / (2.0 * n);
                        let b = theta.sin() * su;
                        let c = theta.cos() * cu;
                        let c = (b * b) + (c * c);
                        let s = a2 + (2.0 * a * b) + c;
                        entry.a = 1.0 / (4.0 * s);
                        entry.d1 = (2.0 * (c - a2)) / s;
                        entry.d2 = -(a2 - (2.0 * a * b) + c) / s;
                    }
                }
            }

            pub fn run(state: &mut State, sample: f64, ctx: &SignalCtx) -> f64 {
                state.run::<UpdateBuffer, HighPass>(sample, ctx)
            }
        }
    }
}

pub struct LowPassButterworth(biquad_filter::butterworth::State);

/// Included for consistency with `LowPassChebyshev`
pub struct LowPassButterworthBuilder(LowPassButterworth);

impl LowPassButterworthBuilder {
    pub fn build(self) -> LowPassButterworth {
        self.0
    }
}

impl LowPassButterworth {
    pub fn new(cutoff_hz: impl Into<Sf64>) -> Self {
        LowPassButterworth(biquad_filter::butterworth::State {
            half_power_frequency_hz: cutoff_hz.into(),
            buffer: biquad_filter::Buffer::new(1),
        })
    }

    pub fn builder(cutoff_hz: impl Into<Sf64>) -> LowPassButterworthBuilder {
        LowPassButterworthBuilder(Self::new(cutoff_hz))
    }
}

impl Filter for LowPassButterworth {
    type Input = f64;
    type Output = f64;

    fn run(&mut self, input: Self::Input, ctx: &crate::signal::SignalCtx) -> Self::Output {
        biquad_filter::butterworth::low_pass::run(&mut self.0, input, ctx)
    }
}

pub struct HighPassButterworth(biquad_filter::butterworth::State);

/// Included for consistency with `HighPassChebyshev`
pub struct HighPassButterworthBuilder(HighPassButterworth);

impl HighPassButterworthBuilder {
    pub fn build(self) -> HighPassButterworth {
        self.0
    }
}

impl HighPassButterworth {
    pub fn new(cutoff_hz: impl Into<Sf64>) -> Self {
        Self(biquad_filter::butterworth::State {
            half_power_frequency_hz: cutoff_hz.into(),
            buffer: biquad_filter::Buffer::new(1),
        })
    }

    pub fn builder(cutoff_hz: impl Into<Sf64>) -> HighPassButterworthBuilder {
        HighPassButterworthBuilder(Self::new(cutoff_hz))
    }
}

impl Filter for HighPassButterworth {
    type Input = f64;
    type Output = f64;

    fn run(&mut self, input: Self::Input, ctx: &crate::signal::SignalCtx) -> Self::Output {
        biquad_filter::butterworth::high_pass::run(&mut self.0, input, ctx)
    }
}

pub struct LowPassChebyshev(biquad_filter::chebyshev::State);

impl LowPassChebyshev {
    pub fn new(cutoff_hz: impl Into<Sf64>, resonance: impl Into<Sf64>) -> Self {
        Self(biquad_filter::chebyshev::State {
            cutoff_hz: cutoff_hz.into(),
            epsilon: resonance.into(),
            buffer: biquad_filter::Buffer::new(1),
        })
    }

    pub fn builder(cutoff_hz: impl Into<Sf64>) -> LowPassChebyshevBuilder {
        LowPassChebyshevBuilder::new(cutoff_hz)
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
            self.resonance
                .unwrap_or_else(|| const_(biquad_filter::chebyshev::EPSILON_MIN)),
        )
    }
}

impl Filter for LowPassChebyshev {
    type Input = f64;
    type Output = f64;

    fn run(&mut self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        biquad_filter::chebyshev::low_pass::run(&mut self.0, input, ctx)
    }
}

impl From<LowPassChebyshevBuilder> for LowPassChebyshev {
    fn from(value: LowPassChebyshevBuilder) -> Self {
        value.build()
    }
}

pub struct HighPassChebyshev(biquad_filter::chebyshev::State);

impl HighPassChebyshev {
    pub fn new(cutoff_hz: impl Into<Sf64>, resonance: impl Into<Sf64>) -> Self {
        Self(biquad_filter::chebyshev::State {
            cutoff_hz: cutoff_hz.into(),
            epsilon: resonance.into(),
            buffer: biquad_filter::Buffer::new(1),
        })
    }

    pub fn builder(cutoff_hz: impl Into<Sf64>) -> HighPassChebyshevBuilder {
        HighPassChebyshevBuilder::new(cutoff_hz)
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
            self.resonance
                .unwrap_or_else(|| const_(biquad_filter::chebyshev::EPSILON_MIN)),
        )
    }
}

impl Filter for HighPassChebyshev {
    type Input = f64;
    type Output = f64;

    fn run(&mut self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        biquad_filter::chebyshev::high_pass::run(&mut self.0, input, ctx)
    }
}

impl From<HighPassChebyshevBuilder> for HighPassChebyshev {
    fn from(value: HighPassChebyshevBuilder) -> Self {
        value.build()
    }
}

pub struct Saturate {
    pub scale: Sf64,
    pub max: Sf64,
    pub min: Sf64,
}

pub struct SaturateBuilder {
    scale: Option<Sf64>,
    max: Option<Sf64>,
    min: Option<Sf64>,
}

impl Saturate {
    pub fn builder() -> SaturateBuilder {
        SaturateBuilder::new()
    }
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
        self.min = Some(threshold);
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

impl Filter for Saturate {
    type Input = f64;
    type Output = f64;

    fn run(&mut self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        let scale = self.scale.sample(ctx);
        let min = self.min.sample(ctx);
        let max = self.max.sample(ctx);
        (input * scale).clamp(min, max)
    }
}
