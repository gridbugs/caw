use crate::signal::{Filter, Sf64, SignalCtx, Trigger};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
};

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

pub struct LowPassButterworth(RefCell<biquad_filter::butterworth::State>);

impl LowPassButterworth {
    pub fn new(cutoff_hz: impl Into<Sf64>) -> Self {
        LowPassButterworth(RefCell::new(biquad_filter::butterworth::State {
            half_power_frequency_hz: cutoff_hz.into(),
            buffer: biquad_filter::Buffer::new(1),
        }))
    }
}

impl Filter for LowPassButterworth {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        biquad_filter::butterworth::low_pass::run(&mut self.0.borrow_mut(), input, ctx)
    }
}

pub struct HighPassButterworth(RefCell<biquad_filter::butterworth::State>);

impl HighPassButterworth {
    pub fn new(cutoff_hz: impl Into<Sf64>) -> Self {
        Self(RefCell::new(biquad_filter::butterworth::State {
            half_power_frequency_hz: cutoff_hz.into(),
            buffer: biquad_filter::Buffer::new(1),
        }))
    }
}

impl Filter for HighPassButterworth {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        biquad_filter::butterworth::high_pass::run(&mut self.0.borrow_mut(), input, ctx)
    }
}

pub struct LowPassChebyshev(RefCell<biquad_filter::chebyshev::State>);

impl LowPassChebyshev {
    pub fn new(cutoff_hz: impl Into<Sf64>, resonance: impl Into<Sf64>) -> Self {
        Self(RefCell::new(biquad_filter::chebyshev::State {
            cutoff_hz: cutoff_hz.into(),
            epsilon: resonance.into(),
            buffer: biquad_filter::Buffer::new(1),
        }))
    }
}

impl Filter for LowPassChebyshev {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        biquad_filter::chebyshev::low_pass::run(&mut self.0.borrow_mut(), input, ctx)
    }
}

pub struct HighPassChebyshev(RefCell<biquad_filter::chebyshev::State>);

impl HighPassChebyshev {
    pub fn new(cutoff_hz: impl Into<Sf64>, resonance: impl Into<Sf64>) -> Self {
        Self(RefCell::new(biquad_filter::chebyshev::State {
            cutoff_hz: cutoff_hz.into(),
            epsilon: resonance.into(),
            buffer: biquad_filter::Buffer::new(1),
        }))
    }
}

impl Filter for HighPassChebyshev {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        biquad_filter::chebyshev::high_pass::run(&mut self.0.borrow_mut(), input, ctx)
    }
}

mod moog_ladder_low_pass_filter {
    use crate::signal::{Filter, Sf64, SignalCtx};
    use std::{cell::RefCell, f64::consts::PI};

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
            s.sample_rate_hz = 44100.0;
            s.saturation = 1.0;
            s.q = 3.0;
            s.set_cutoff_hz(1000.0);
            s.set_resonance(0.0);
            s
        }

        fn set_resonance(&mut self, resonance: f64) {
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
        state: RefCell<OberheimVariationMoogState>,
        cutoff_hz: Sf64,
        resonance: Sf64,
    }

    impl LowPassMoogLadder {
        pub fn new(cutoff_hz: impl Into<Sf64>, resonance: impl Into<Sf64>) -> Self {
            Self {
                state: RefCell::new(OberheimVariationMoogState::new()),
                cutoff_hz: cutoff_hz.into(),
                resonance: resonance.into(),
            }
        }
    }

    impl Filter for LowPassMoogLadder {
        type Input = f64;
        type Output = f64;

        fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
            let cutoff_hz = self.cutoff_hz.sample(ctx);
            let resonance = self.resonance.sample(ctx);
            let mut state = self.state.borrow_mut();
            if state.sample_rate_hz == ctx.sample_rate_hz {
                if cutoff_hz != state.cutoff_hz {
                    state.set_cutoff_hz(cutoff_hz);
                }
            } else {
                state.sample_rate_hz = ctx.sample_rate_hz;
                state.set_cutoff_hz(cutoff_hz);
            }
            if resonance != state.resonance {
                state.set_resonance(resonance);
            }
            state.process_sample(input)
        }
    }
}

pub use moog_ladder_low_pass_filter::*;

pub struct Saturate {
    pub scale: Sf64,
    pub max: Sf64,
    pub min: Sf64,
}

impl Filter for Saturate {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        let scale = self.scale.sample(ctx);
        let min = self.min.sample(ctx);
        let max = self.max.sample(ctx);
        (input * scale).clamp(min, max)
    }
}

pub struct Compress {
    pub threshold: Sf64,
    pub ratio: Sf64,
    pub scale: Sf64,
}

impl Filter for Compress {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        let input = input * self.scale.sample(ctx);
        let input_abs = input.abs();
        let threshold = self.threshold.sample(ctx);
        if input_abs > threshold {
            let delta = input_abs - threshold;
            let scaled_delta = delta * self.ratio.sample(ctx);
            (threshold + scaled_delta) * input.signum()
        } else {
            input
        }
    }
}

pub struct Delay {
    samples: RefCell<VecDeque<f64>>,
    time_s: Sf64,
}

impl Delay {
    pub fn new(time_s: Sf64) -> Self {
        Self {
            samples: RefCell::new(VecDeque::new()),
            time_s,
        }
    }
}

impl Filter for Delay {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        let target_size = (self.time_s.sample(ctx) * ctx.sample_rate_hz) as usize;
        let mut samples = self.samples.borrow_mut();
        if samples.len() < target_size {
            samples.push_back(input);
            0.0
        } else {
            if samples.len() == target_size {
                samples.push_back(input);
            }
            samples.pop_front().unwrap_or(0.0)
        }
    }
}

pub struct Echo {
    delay: Delay,
    scale: Sf64,
    previous_sample: Cell<f64>,
}

impl Echo {
    pub fn new(delay_s: Sf64, scale: Sf64) -> Self {
        Self {
            delay: Delay::new(delay_s),
            scale,
            previous_sample: Cell::new(0.0),
        }
    }
}

impl Filter for Echo {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        let scale = self.scale.sample(ctx);
        let delay_input = (input + self.previous_sample.get()) * scale;
        let delay_output = self.delay.run(delay_input, ctx);
        self.previous_sample.set(delay_output);
        input + delay_output
    }
}

pub struct SampleAndHold {
    trigger: Trigger,
    sample: Cell<f64>,
}

impl SampleAndHold {
    pub fn new(trigger: Trigger) -> Self {
        Self {
            trigger,
            sample: Cell::new(0.0),
        }
    }
}

impl Filter for SampleAndHold {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        if self.trigger.sample(ctx) {
            self.sample.set(input);
        }
        self.sample.get()
    }
}

pub struct Quantize {
    pub resolution: Sf64,
}

impl Quantize {
    pub fn new(resolution: impl Into<Sf64>) -> Self {
        Self {
            resolution: resolution.into(),
        }
    }
}

impl Filter for Quantize {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        let resolution = self.resolution.sample(ctx);
        ((input * resolution) as i64) as f64 / resolution
    }
}

pub struct DownSample {
    scale: Sf64,
    last_sample: Cell<f64>,
    until_next_sample: Cell<f64>,
}

impl DownSample {
    pub fn new(scale: impl Into<Sf64>) -> Self {
        Self {
            scale: scale.into(),
            last_sample: Cell::new(0.0),
            until_next_sample: Cell::new(0.0),
        }
    }
}

impl Filter for DownSample {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        let period = self.scale.sample(ctx).max(1.0).recip();
        let until_next_sample = self.until_next_sample.get() - period;
        if until_next_sample <= 0.0 {
            self.until_next_sample.set(1.0);
            self.last_sample.set(input);
            input
        } else {
            self.until_next_sample.set(until_next_sample);
            self.last_sample.get()
        }
    }
}
