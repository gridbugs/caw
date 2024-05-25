use crate::{
    biquad_filter, freeverb, moog_ladder_low_pass_filter,
    signal::{freq_hz, Filter, Freq, Sf64, Sfreq, SignalCtx, Trigger},
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
};

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
        self.until_next_sample
            .set(self.until_next_sample.get() - period);
        if self.until_next_sample.get() < 0.0 {
            while self.until_next_sample.get() < 0.0 {
                self.until_next_sample
                    .set(self.until_next_sample.get() + 1.0);
            }
            self.last_sample.set(input);
            input
        } else {
            self.last_sample.get()
        }
    }
}

pub struct QuantizeToScale {
    pub notes: Vec<Sfreq>,
}

impl QuantizeToScale {
    pub fn new(notes: Vec<Sfreq>) -> Self {
        assert!(!notes.is_empty(), "notes may not be empty");
        Self { notes }
    }

    fn quantize_to_note(freq_hz: f64, note_base_freq_hz: f64) -> f64 {
        let mut note_freq_hz = note_base_freq_hz;
        loop {
            let next_note_freq_hz = note_freq_hz * 2.0;
            if next_note_freq_hz > freq_hz {
                return note_freq_hz;
            }
            note_freq_hz = next_note_freq_hz;
        }
    }
}

impl Filter for QuantizeToScale {
    type Input = Freq;
    type Output = Freq;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        let input_hz = input.hz();
        let mut best = Self::quantize_to_note(input_hz, self.notes[0].sample_hz(ctx));
        let mut best_delta = input_hz - best;
        for note in &self.notes[1..] {
            let quantized = Self::quantize_to_note(input_hz, note.sample_hz(ctx));
            let delta = input_hz - quantized;
            if delta < best_delta {
                best_delta = delta;
                best = quantized;
            }
        }
        freq_hz(best)
    }
}

pub struct Reverb {
    freeverb: RefCell<freeverb::ReverbModel>,
    room_size: Sf64,
    room_size_prev: Cell<f64>,
    damping: Sf64,
    damping_prev: Cell<f64>,
}

impl Reverb {
    pub const DEFAULT_ROOM_SIZE: f64 = freeverb::INITIAL_ROOM_SIZE;
    pub const DEFAULT_DAMPING: f64 = freeverb::INITIAL_DAMPING;
    pub fn new(room_size: Sf64, damping: Sf64) -> Self {
        Self {
            freeverb: RefCell::new(freeverb::ReverbModel::new()),
            room_size,
            room_size_prev: Self::DEFAULT_ROOM_SIZE.into(),
            damping,
            damping_prev: Self::DEFAULT_DAMPING.into(),
        }
    }
}

impl Filter for Reverb {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        let mut freeverb = self.freeverb.borrow_mut();
        let room_size = self.room_size.sample(ctx);
        if room_size != self.room_size_prev.get() {
            self.room_size_prev.set(room_size);
            freeverb.set_room_size(room_size);
        }
        let damping = self.damping.sample(ctx);
        if damping != self.damping_prev.get() {
            self.damping_prev.set(damping);
            freeverb.set_damping(damping);
        }
        freeverb.process(input)
    }
}

pub struct EnvelopeFollower {
    low_pass_filter: LowPassButterworth,
}

impl EnvelopeFollower {
    pub const DEFAULT_SENSITIVITY_HZ: f64 = 60.0;
    pub fn new(sensitivity_hz: impl Into<Sf64>) -> Self {
        Self {
            low_pass_filter: LowPassButterworth::new(sensitivity_hz),
        }
    }
}

impl Filter for EnvelopeFollower {
    type Input = f64;
    type Output = f64;

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output {
        self.low_pass_filter.run(input.abs(), ctx)
    }
}
