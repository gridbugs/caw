// This is based on the filter designs at:
// https://exstrom.com/journal/sigproc/dsigproc.html
// This module will take parameter names from the reference implementation for easier
// correspondence between the two implementations, but the public API to this module will
// change the names for consistency.

#[derive(Default)]
struct BufferEntry {
    a: f32,
    d1: f32,
    d2: f32,
    w0: f32,
    w1: f32,
    w2: f32,
}

struct Buffer {
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

    fn apply_low_pass(&mut self, mut sample: f32) -> f32 {
        for entry in self.entries.iter_mut() {
            entry.w0 = (entry.d1 * entry.w1) + (entry.d2 * entry.w2) + sample;
            sample = entry.a * (entry.w0 + (2.0 * entry.w1) + entry.w2);
            entry.w2 = entry.w1;
            entry.w1 = entry.w0;
        }
        sample
    }

    fn apply_high_pass(&mut self, mut sample: f32) -> f32 {
        for entry in self.entries.iter_mut() {
            entry.w0 = (entry.d1 * entry.w1) + (entry.d2 * entry.w2) + sample;
            sample = entry.a * (entry.w0 - (2.0 * entry.w1) + entry.w2);
            entry.w2 = entry.w1;
            entry.w1 = entry.w0;
        }
        sample
    }
}

trait PassTrait {
    fn apply(buffer: &mut Buffer, sample: f32) -> f32;
}
struct LowPass;
struct HighPass;
impl PassTrait for LowPass {
    fn apply(buffer: &mut Buffer, sample: f32) -> f32 {
        buffer.apply_low_pass(sample)
    }
}
impl PassTrait for HighPass {
    fn apply(buffer: &mut Buffer, sample: f32) -> f32 {
        buffer.apply_high_pass(sample)
    }
}

pub mod butterworth {
    use super::*;

    trait UpdateBufferTrait {
        fn update_entries(buffer: &mut Buffer, half_power_frequency_hz: f32);
    }

    pub struct State {
        buffer: Buffer,
        prev_half_power_frequency_hz: f32,
    }

    impl State {
        pub fn new(filter_order_half: usize) -> Self {
            Self {
                buffer: Buffer::new(filter_order_half),
                prev_half_power_frequency_hz: 0.0,
            }
        }

        fn run<U: UpdateBufferTrait, P: PassTrait>(
            &mut self,
            sample: f32,
            sample_rate_hz: f32,
            half_power_frequency_hz: f32,
        ) -> f32 {
            let half_power_frequency_hz = half_power_frequency_hz.max(0.0);
            if self.buffer.entries.is_empty() {
                return sample;
            }
            if half_power_frequency_hz != self.prev_half_power_frequency_hz {
                self.prev_half_power_frequency_hz = half_power_frequency_hz;
                let half_power_frequency_sample_rate_ratio =
                    half_power_frequency_hz / sample_rate_hz;
                U::update_entries(
                    &mut self.buffer,
                    half_power_frequency_sample_rate_ratio,
                );
            }
            P::apply(&mut self.buffer, sample)
        }
    }

    pub mod low_pass {
        use super::*;
        use std::f32::consts::PI;

        struct UpdateBuffer;
        impl UpdateBufferTrait for UpdateBuffer {
            fn update_entries(
                buffer: &mut Buffer,
                half_power_frequency_sample_rate_ratio: f32,
            ) {
                let a = (PI * half_power_frequency_sample_rate_ratio).tan();
                let a2 = a * a;
                let n = buffer.entries.len() as f32;
                for (i, entry) in buffer.entries.iter_mut().enumerate() {
                    let r = ((PI * ((2.0 * i as f32) + 1.0)) / (4.0 * n)).sin();
                    let s = a2 + (2.0 * a * r) + 1.0;
                    entry.a = a2 / s;
                    entry.d1 = (2.0 * (1.0 - a2)) / s;
                    entry.d2 = -(a2 - (2.0 * a * r) + 1.0) / s;
                }
            }
        }

        /// Run the butterworth low-pass filter for a single sample.
        pub fn run(
            state: &mut State,
            sample: f32,
            sample_rate_hz: f32,
            half_power_frequency_hz: f32,
        ) -> f32 {
            state.run::<UpdateBuffer, LowPass>(
                sample,
                sample_rate_hz,
                half_power_frequency_hz,
            )
        }
    }

    pub mod high_pass {
        use super::*;
        use std::f32::consts::PI;

        struct UpdateBuffer;
        impl UpdateBufferTrait for UpdateBuffer {
            fn update_entries(
                buffer: &mut Buffer,
                half_power_frequency_sample_rate_ratio: f32,
            ) {
                let a = (PI * half_power_frequency_sample_rate_ratio).tan();
                let a2 = a * a;
                let n = buffer.entries.len() as f32;
                for (i, entry) in buffer.entries.iter_mut().enumerate() {
                    let r = ((PI * ((2.0 * i as f32) + 1.0)) / (4.0 * n)).sin();
                    let s = a2 + (2.0 * a * r) + 1.0;
                    entry.a = 1.0 / s;
                    entry.d1 = (2.0 * (1.0 - a2)) / s;
                    entry.d2 = -(a2 - (2.0 * a * r) + 1.0) / s;
                }
            }
        }

        /// Run the butterworth high-pass filter for a single sample.
        pub fn run(
            state: &mut State,
            sample: f32,
            sample_rate_hz: f32,
            half_power_frequency_hz: f32,
        ) -> f32 {
            state.run::<UpdateBuffer, HighPass>(
                sample,
                sample_rate_hz,
                half_power_frequency_hz,
            )
        }
    }
}

pub mod chebyshev {
    use super::*;

    pub const EPSILON_MIN: f32 = 0.01;

    trait UpdateBufferTrait {
        fn update_entries(buffer: &mut Buffer, cutoff_hz: f32, epsilon: f32);
    }

    pub struct State {
        buffer: Buffer,
        prev_cutoff_hz: f32,
        prev_epsilon: f32,
    }

    impl State {
        pub fn new(filter_order_half: usize) -> Self {
            Self {
                buffer: Buffer::new(filter_order_half),
                prev_cutoff_hz: 0.0,
                prev_epsilon: 0.0,
            }
        }

        fn run<U: UpdateBufferTrait, P: PassTrait>(
            &mut self,
            sample: f32,
            sample_rate_hz: f32,
            cutoff_hz: f32,
            epsilon: f32,
        ) -> f32 {
            let cutoff_hz = cutoff_hz.max(0.0);
            if self.buffer.entries.is_empty() {
                return sample;
            }
            let cutoff_sample_rate_ratio = cutoff_hz / sample_rate_hz;
            let epsilon = epsilon.max(EPSILON_MIN);
            if cutoff_hz != self.prev_cutoff_hz || epsilon != self.prev_epsilon
            {
                self.prev_cutoff_hz = cutoff_hz;
                self.prev_epsilon = epsilon;
                U::update_entries(
                    &mut self.buffer,
                    cutoff_sample_rate_ratio,
                    epsilon,
                );
            }
            let output_scaled = P::apply(&mut self.buffer, sample);
            let scale_factor = (1.0 - (-epsilon).exp()) / 2.0;
            output_scaled / scale_factor
        }
    }

    pub mod low_pass {
        use super::*;
        use std::f32::consts::PI;

        struct UpdateBuffer;
        impl UpdateBufferTrait for UpdateBuffer {
            fn update_entries(
                buffer: &mut Buffer,
                cutoff_sample_rate_ratio: f32,
                epsilon: f32,
            ) {
                let a = (PI * cutoff_sample_rate_ratio).tan();
                let a2 = a * a;
                let u =
                    ((1.0 + (1.0 + (epsilon * epsilon)).sqrt()) / epsilon).ln();
                let n = (buffer.entries.len() * 2) as f32;
                let su = (u / n).sinh();
                let cu = (u / n).cosh();
                for (i, entry) in buffer.entries.iter_mut().enumerate() {
                    let theta = (PI * ((2.0 * i as f32) + 1.0)) / (2.0 * n);
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

        /// Run the chebyshev low-pass filter for a single sample.
        pub fn run(
            state: &mut State,
            sample: f32,
            sample_rate_hz: f32,
            cutoff_hz: f32,
            epsilon: f32,
        ) -> f32 {
            state.run::<UpdateBuffer, LowPass>(
                sample,
                sample_rate_hz,
                cutoff_hz,
                epsilon,
            )
        }
    }

    pub mod high_pass {
        use super::*;
        use std::f32::consts::PI;

        struct UpdateBuffer;
        impl UpdateBufferTrait for UpdateBuffer {
            fn update_entries(
                buffer: &mut Buffer,
                cutoff_sample_rate_ratio: f32,
                epsilon: f32,
            ) {
                let a = (PI * cutoff_sample_rate_ratio).tan();
                let a2 = a * a;
                let u =
                    ((1.0 + (1.0 + (epsilon * epsilon)).sqrt()) / epsilon).ln();
                let n = (buffer.entries.len() * 2) as f32;
                let su = (u / n).sinh();
                let cu = (u / n).cosh();
                for (i, entry) in buffer.entries.iter_mut().enumerate() {
                    let theta = (PI * ((2.0 * i as f32) + 1.0)) / (2.0 * n);
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

        /// Run the chebyshev high-pass filter for a single sample.
        pub fn run(
            state: &mut State,
            sample: f32,
            sample_rate_hz: f32,
            cutoff_hz: f32,
            epsilon: f32,
        ) -> f32 {
            state.run::<UpdateBuffer, HighPass>(
                sample,
                sample_rate_hz,
                cutoff_hz,
                epsilon,
            )
        }
    }
}
