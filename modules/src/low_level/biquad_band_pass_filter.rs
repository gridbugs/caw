// This is based on the filter designs at:
// https://exstrom.com/journal/sigproc/dsigproc.html
// This module will take parameter names from the reference implementation for easier
// correspondence between the two implementations, but the public API to this module will
// change the names for consistency.

// This is separate from the biquad_filter module because unlike high pass and low pass filters it
// doesn't use the same common machinery.

#[derive(Default, Debug)]
struct BufferEntry {
    a: f64,
    d1: f64,
    d2: f64,
    d3: f64,
    d4: f64,
    w0: f64,
    w1: f64,
    w2: f64,
    w3: f64,
    w4: f64,
}

#[derive(Debug)]
pub struct Buffer {
    entries: Vec<BufferEntry>,
}

impl Buffer {
    pub fn new(filter_order_quarter: usize) -> Self {
        let mut entries = Vec::new();
        for _ in 0..filter_order_quarter {
            entries.push(Default::default());
        }
        Self { entries }
    }

    fn apply(&mut self, mut sample: f64) -> f64 {
        for entry in self.entries.iter_mut() {
            entry.w0 = (entry.d1 * entry.w1)
                + (entry.d2 * entry.w2)
                + (entry.d3 * entry.w3)
                + (entry.d4 * entry.w4)
                + sample;
            sample = entry.a * (entry.w0 - (2.0 * entry.w2) + entry.w4);
            entry.w4 = entry.w3;
            entry.w3 = entry.w2;
            entry.w2 = entry.w1;
            entry.w1 = entry.w0;
        }
        sample
    }

    fn reset(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = Default::default()
        }
    }
}

pub mod butterworth {
    use super::*;
    use std::f64::consts::PI;

    pub struct State {
        buffer: Buffer,
        prev_lower_half_power_frequency_hz: f64,
        prev_upper_half_power_frequency_hz: f64,
    }

    impl State {
        pub fn new(filter_order_quarter: usize) -> Self {
            Self {
                buffer: Buffer::new(filter_order_quarter),
                prev_lower_half_power_frequency_hz: 0.0,
                prev_upper_half_power_frequency_hz: 0.0,
            }
        }

        fn reset(&mut self) {
            self.buffer.reset();
            self.prev_lower_half_power_frequency_hz = 0.0;
            self.prev_upper_half_power_frequency_hz = 0.0;
        }

        pub fn run(
            &mut self,
            sample: f64,
            sample_rate_hz: f64,
            lower_half_power_frequency_hz: f64,
            upper_half_power_frequency_hz: f64,
        ) -> f64 {
            let lower_half_power_frequency_hz =
                lower_half_power_frequency_hz.max(0.0);
            let upper_half_power_frequency_hz = upper_half_power_frequency_hz
                .max(lower_half_power_frequency_hz);
            if self.buffer.entries.is_empty() {
                return sample;
            }
            if lower_half_power_frequency_hz
                != self.prev_lower_half_power_frequency_hz
                || upper_half_power_frequency_hz
                    != self.prev_upper_half_power_frequency_hz
            {
                self.prev_lower_half_power_frequency_hz =
                    lower_half_power_frequency_hz;
                self.prev_upper_half_power_frequency_hz =
                    upper_half_power_frequency_hz;
                update_entries(
                    &mut self.buffer,
                    lower_half_power_frequency_hz,
                    upper_half_power_frequency_hz,
                    sample_rate_hz,
                );
            }
            let out = self.buffer.apply(sample);
            if out.is_nan() {
                // This filter is prone to exploding when the cutoffs are too low or too close
                // together. Recover back to the initial state in such an occurrence.
                self.reset();
                0.0
            } else {
                out
            }
        }
    }

    fn update_entries(
        buffer: &mut Buffer,
        lower_half_power_frequency_hz: f64,
        upper_half_power_frequency_hz: f64,
        sample_rate_hz: f64,
    ) {
        let a = ((PI
            * (lower_half_power_frequency_hz + upper_half_power_frequency_hz))
            / sample_rate_hz)
            .cos()
            / ((PI
                * (upper_half_power_frequency_hz
                    - lower_half_power_frequency_hz))
                / sample_rate_hz)
                .cos();
        let a2 = a * a;
        let b = ((PI
            * (upper_half_power_frequency_hz - lower_half_power_frequency_hz))
            / sample_rate_hz)
            .tan();
        let b2 = b * b;
        let n = buffer.entries.len() as f64;
        for (i, entry) in buffer.entries.iter_mut().enumerate() {
            let r = ((PI * ((2.0 * i as f64) + 1.0)) / (4.0 * n)).sin();
            let s = b2 + (2.0 * b * r) + 1.0;
            entry.a = b2 / s;
            entry.d1 = 4.0 * a * (1.0 + (b * r)) / s;
            entry.d2 = 2.0 * (b2 - (2.0 * a2) - 1.0) / s;
            entry.d3 = 4.0 * a * (1.0 - (b * r)) / s;
            entry.d4 = -(b2 - (2.0 * b * r) + 1.0) / s;
        }
    }
}

pub mod chebyshev {
    use super::*;
    use std::f64::consts::PI;

    pub const EPSILON_MIN: f64 = 0.01;

    pub struct State {
        buffer: Buffer,
        prev_lower_cutoff_hz: f64,
        prev_upper_cutoff_hz: f64,
        prev_epsilon: f64,
    }

    impl State {
        pub fn new(filter_order_quarter: usize) -> Self {
            Self {
                buffer: Buffer::new(filter_order_quarter),
                prev_lower_cutoff_hz: 0.0,
                prev_upper_cutoff_hz: 0.0,
                prev_epsilon: 0.0,
            }
        }

        fn reset(&mut self) {
            self.buffer.reset();
            self.prev_lower_cutoff_hz = 0.0;
            self.prev_upper_cutoff_hz = 0.0;
        }

        pub fn run(
            &mut self,
            sample: f64,
            sample_rate_hz: f64,
            lower_cutoff_hz: f64,
            upper_cutoff_hz: f64,
            epsilon: f64,
        ) -> f64 {
            if self.buffer.entries.is_empty() {
                return sample;
            }
            let epsilon = epsilon.max(EPSILON_MIN);
            if lower_cutoff_hz != self.prev_lower_cutoff_hz
                || upper_cutoff_hz != self.prev_upper_cutoff_hz
                || epsilon != self.prev_epsilon
            {
                self.prev_lower_cutoff_hz = lower_cutoff_hz;
                self.prev_upper_cutoff_hz = upper_cutoff_hz;
                self.prev_epsilon = epsilon;
                update_entries(
                    &mut self.buffer,
                    lower_cutoff_hz,
                    upper_cutoff_hz,
                    epsilon,
                    sample_rate_hz,
                );
            }
            let output_scaled = self.buffer.apply(sample);
            let scale_factor = (1.0 - (-epsilon).exp()) / 2.0;
            let out = output_scaled / scale_factor;
            if out.is_nan() {
                // This filter is prone to exploding when the cutoffs are too low or too close
                // together. Recover back to the initial state in such an occurrence.
                self.reset();
                0.0
            } else {
                out
            }
        }
    }

    fn update_entries(
        buffer: &mut Buffer,
        lower_cutoff_hz: f64,
        upper_cutoff_hz: f64,
        epsilon: f64,
        sample_rate_hz: f64,
    ) {
        let a = ((PI * (lower_cutoff_hz + upper_cutoff_hz)) / sample_rate_hz)
            .cos()
            / ((PI * (upper_cutoff_hz - lower_cutoff_hz)) / sample_rate_hz)
                .cos();
        let a2 = a * a;
        let b =
            ((PI * (upper_cutoff_hz - lower_cutoff_hz)) / sample_rate_hz).tan();
        let b2 = b * b;
        let u = ((1.0 + (1.0 + (epsilon * epsilon)).sqrt()) / epsilon).ln();
        let n = (buffer.entries.len() * 4) as f64;
        let su = ((2.0 * u) / n).sinh();
        let cu = ((2.0 * u) / n).cosh();
        for (i, entry) in buffer.entries.iter_mut().enumerate() {
            let theta = (PI * ((2.0 * i as f64) + 1.0)) / n;
            let r = theta.sin() * su;
            let c = theta.cos() * cu;
            let c = (r * r) + (c * c);
            let s = (b2 * c) + (2.0 * b * r) + 1.0;
            entry.a = b2 / (4.0 * s);
            entry.d1 = 4.0 * a * (1.0 + (b * r)) / s;
            entry.d2 = 2.0 * ((b2 * c) - (2.0 * a2) - 1.0) / s;
            entry.d3 = 4.0 * a * (1.0 - (b * r)) / s;
            entry.d4 = -((b2 * c) - (2.0 * b * r) + 1.0) / s;
        }
    }
}
