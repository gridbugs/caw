use std::{f64, marker::PhantomData};

#[derive(Default, Debug, Clone, Copy, PartialEq)]
struct Params {
    cutoff_hz: f64,
    resonance: f64,
}

struct Common {
    cos_omega: f64,
    alpha: f64,
}

impl Common {
    fn from_params(
        Params {
            cutoff_hz,
            resonance,
        }: Params,
        sample_rate_hz: f64,
    ) -> Self {
        let omega = (f64::consts::PI * 2.0 * cutoff_hz) / sample_rate_hz;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * resonance);
        Self { cos_omega, alpha }
    }
}

#[derive(Default, Debug)]
struct NormalizedCoefficients {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

trait FilterType {
    fn normalized_coefficients(
        params: Params,
        sample_rate_hz: f64,
    ) -> NormalizedCoefficients;
}

struct LowPass;

impl FilterType for LowPass {
    fn normalized_coefficients(
        params: Params,
        sample_rate_hz: f64,
    ) -> NormalizedCoefficients {
        let Common { cos_omega, alpha } =
            Common::from_params(params, sample_rate_hz);
        let a0 = 1.0 + alpha;
        NormalizedCoefficients {
            b0: ((1.0 - cos_omega) * 0.5) / a0,
            b1: (1.0 - cos_omega) / a0,
            b2: ((1.0 - cos_omega) * 0.5) / a0,
            a1: (-2.0 * cos_omega) / a0,
            a2: (1.0 - alpha) / a0,
        }
    }
}

#[derive(Default)]
struct State {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl State {
    fn process(
        &mut self,
        sample: f64,
        &NormalizedCoefficients { b0, b1, b2, a1, a2 }: &NormalizedCoefficients,
    ) -> f64 {
        let output = (b0 * sample) + (b1 * self.x1) + (b2 * self.x2)
            - (a1 * self.y1)
            - (a2 * self.y2);
        self.x2 = self.x1;
        self.x1 = sample;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

struct BiquadFilter<T: FilterType> {
    filter_type: PhantomData<T>,
    state: State,
    normalized_coefficients: NormalizedCoefficients,
    params: Params,
}

impl<T: FilterType> BiquadFilter<T> {
    fn process(
        &mut self,
        sample: f64,
        cutoff_hz: f64,
        resonance: f64,
        sample_rate_hz: f64,
    ) -> f64 {
        // Resonance range
        const Q_MIN: f64 = 0.70710678;
        const Q_MAX: f64 = 4.0;
        let params = Params {
            cutoff_hz: cutoff_hz.clamp(0., 20_000.0),
            resonance: Q_MIN + ((Q_MAX - Q_MIN) * resonance),
        };
        if params != self.params {
            self.params = params;
            self.normalized_coefficients =
                T::normalized_coefficients(params, sample_rate_hz);
        }
        self.state.process(sample, &self.normalized_coefficients)
    }
}

pub struct BiquadFilterLowPass(BiquadFilter<LowPass>);

impl BiquadFilterLowPass {
    pub fn new() -> Self {
        Self(BiquadFilter {
            filter_type: PhantomData,
            state: State::default(),
            normalized_coefficients: NormalizedCoefficients::default(),
            params: Params::default(),
        })
    }

    pub fn process(
        &mut self,
        sample: f64,
        cutoff_hz: f64,
        resonance: f64,
        sample_rate_hz: f64,
    ) -> f64 {
        self.0.process(sample, cutoff_hz, resonance, sample_rate_hz)
    }
}
