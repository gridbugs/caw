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

    fn run(&mut self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
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

    fn run(&mut self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
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

mod moog_ladder_low_pass_filter {
    use crate::signal::{const_, Filter, Sf64, SignalCtx};
    use std::f64::consts::PI;

    // This is the Oberheim Variation Model implementation of the Moog Ladder low pass filter. It's
    // based on a reference implementation by Will Pirkle which can be found here:
    // https://github.com/ddiakopoulos/MoogLadders/blob/master/src/OberheimVariationModel.h

    struct VAOnePole {
        alpha: f64,
        beta: f64,
        gamma: f64,
        delta: f64,
        epsilon: f64,
        a0: f64,
        feedback: f64,
        z1: f64,
    }

    impl Default for VAOnePole {
        fn default() -> Self {
            Self {
                alpha: 1.0,
                beta: 0.0,
                gamma: 1.0,
                delta: 0.0,
                epsilon: 0.0,
                a0: 1.0,
                feedback: 0.0,
                z1: 0.0,
            }
        }
    }

    impl VAOnePole {
        fn feedback_output(&self) -> f64 {
            self.beta * (self.z1 + (self.feedback * self.delta))
        }

        fn tick(&mut self, mut s: f64) -> f64 {
            s = (s * self.gamma) + self.feedback + (self.epsilon * self.feedback_output());
            let vn = ((self.a0 * s) - self.z1) * self.alpha;
            let out = vn + self.z1;
            self.z1 = vn + out;
            out
        }
    }

    #[derive(Default)]
    struct OberheimVariationMoogState {
        lpf1: VAOnePole,
        lpf2: VAOnePole,
        lpf3: VAOnePole,
        lpf4: VAOnePole,
        k: f64,
        gamma: f64,
        alpha0: f64,
        q: f64,
        saturation: f64,
        cutoff_hz: f64,
        resonance: f64,
        sample_rate_hz: f64,
    }

    impl OberheimVariationMoogState {
        fn new() -> Self {
            let mut s = Self::default();
            s.saturation = 1.0;
            s.q = 3.0;
            s.set_cutoff_hz(1000.0);
            s.set_resonance(0.0);
            s
        }

        fn set_resonance(&mut self, resonance: f64) {
            // this maps resonance = 0->1 to K = 0 -> 4
            self.resonance = resonance;
            self.k = resonance * 4.0;
        }

        fn set_cutoff_hz(&mut self, cutoff_hz: f64) {
            self.cutoff_hz = cutoff_hz;
            // prewarp for BZT
            let wd = 2.0 * PI * cutoff_hz;
            let t = 1.0 / self.sample_rate_hz;
            let wa = (2.0 / t) * (wd * t / 2.0).tan();
            let g = wa * t / 2.0;
            let feedforward_coeff = g / (1.0 + g);
            self.lpf1.alpha = feedforward_coeff;
            self.lpf2.alpha = feedforward_coeff;
            self.lpf3.alpha = feedforward_coeff;
            self.lpf4.alpha = feedforward_coeff;
            self.lpf1.beta =
                (feedforward_coeff * feedforward_coeff * feedforward_coeff) / (1.0 + g);
            self.lpf2.beta = (feedforward_coeff * feedforward_coeff) / (1.0 + g);
            self.lpf3.beta = feedforward_coeff / (1.0 + g);
            self.lpf4.beta = 1.0 / (1.0 + g);
            self.gamma =
                feedforward_coeff * feedforward_coeff * feedforward_coeff * feedforward_coeff;
            self.alpha0 = 1.0 / (1.0 + (self.k * self.gamma));
        }

        fn process_sample(&mut self, mut sample: f64) -> f64 {
            let sigma = self.lpf1.feedback_output()
                + self.lpf2.feedback_output()
                + self.lpf3.feedback_output()
                + self.lpf4.feedback_output();
            sample *= 1.0 + self.k;
            // calculate input to first filter
            let u = (sample - (self.k * sigma)) * self.alpha0;
            let u = (self.saturation * u).tanh();
            let stage1 = self.lpf1.tick(u);
            let stage2 = self.lpf2.tick(stage1);
            let stage3 = self.lpf3.tick(stage2);
            let stage4 = self.lpf4.tick(stage3);
            stage4
        }
    }

    pub struct LowPassMoogLadder {
        state: OberheimVariationMoogState,
        cutoff_hz: Sf64,
        resonance: Sf64,
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

    impl LowPassMoogLadder {
        pub fn new(cutoff_hz: impl Into<Sf64>, resonance: impl Into<Sf64>) -> Self {
            Self {
                state: OberheimVariationMoogState::new(),
                cutoff_hz: cutoff_hz.into(),
                resonance: resonance.into(),
            }
        }

        pub fn builder(cutoff_hz: impl Into<Sf64>) -> LowPassMoogLadderBuilder {
            LowPassMoogLadderBuilder::new(cutoff_hz)
        }
    }

    impl Filter for LowPassMoogLadder {
        type Input = f64;
        type Output = f64;

        fn run(&mut self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
            self.state.sample_rate_hz = ctx.sample_rate_hz;
            let cutoff_hz = self.cutoff_hz.sample(ctx);
            let resonance = self.resonance.sample(ctx);
            if cutoff_hz != self.state.cutoff_hz {
                self.state.set_cutoff_hz(cutoff_hz);
            }
            if resonance != self.state.resonance {
                self.state.set_resonance(resonance);
            }
            self.state.process_sample(input)
        }
    }
}

pub use moog_ladder_low_pass_filter::*;
