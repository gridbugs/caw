use rand::{rngs::StdRng, Rng, SeedableRng};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Freq {
    hz: f64,
}

impl Freq {
    pub const fn from_hz(hz: f64) -> Self {
        Self { hz }
    }

    pub fn from_s(s: f64) -> Self {
        Self::from_hz(1.0 / s)
    }

    pub const fn hz(&self) -> f64 {
        self.hz
    }

    pub fn s(&self) -> f64 {
        self.hz() / 1.0
    }
}

pub const fn freq_hz(hz: f64) -> Freq {
    Freq::from_hz(hz)
}

pub fn freq_s(s: f64) -> Freq {
    Freq::from_s(s)
}

pub fn sfreq_hz(hz: impl Into<Sf64>) -> Sfreq {
    hz.into().map(freq_hz)
}

pub fn sfreq_s(s: impl Into<Sf64>) -> Sfreq {
    s.into().map(freq_s)
}

pub fn sfreq_to_hz(sfreq: impl Into<Sfreq>) -> Sf64 {
    sfreq.into().map(|f| f.hz())
}

pub fn sfreq_to_s(sfreq: impl Into<Sfreq>) -> Sf64 {
    sfreq.into().map(|f| f.s())
}

pub fn noise() -> Sf64 {
    let mut rng = StdRng::from_entropy();
    Signal::from_fn(move |_ctx| (rng.gen::<f64>() * 2.0) - 1.0)
}

pub fn noise_01() -> Sf64 {
    let mut rng = StdRng::from_entropy();
    Signal::from_fn(move |_ctx| (rng.gen::<f64>()))
}

/// Information about the current state of playback that will be passed to every signal each frame
pub struct SignalCtx {
    pub sample_index: u64,
    pub sample_rate_hz: f64,
    pub num_threads: usize,
}

/// Low-level interface for signals
pub trait SignalRaw<T> {
    /// Generate a single sample. Implementations can assume that this method will only be called
    /// at most once per frame, though it will not necessarily be called each frame.
    fn sample(&mut self, ctx: &SignalCtx) -> T;
}

pub struct SignalRawFn<T, F: FnMut(&SignalCtx) -> T>(F);
impl<T, F: FnMut(&SignalCtx) -> T> SignalRaw<T> for SignalRawFn<T, F> {
    fn sample(&mut self, ctx: &SignalCtx) -> T {
        (self.0)(ctx)
    }
}

pub fn raw_fn<T, F: FnMut(&SignalCtx) -> T>(f: F) -> SignalRawFn<T, F> {
    SignalRawFn(f)
}

/// Caching layer around a `SignalRaw` which makes sure that its `sample` method is only called at
/// most once per frame. It is "unshared" in the sense that it owns the `SignalRaw` instance that
/// it is wrapping.
struct SignalUnshared<T> {
    signal: Box<dyn SignalRaw<T> + Send>,
    buffered_sample: Option<T>,
    next_sample_index: u64,
}

impl<T: Clone> SignalUnshared<T> {
    fn new<S: SignalRaw<T> + Send + 'static>(signal: S) -> Self {
        Self {
            signal: Box::new(signal),
            buffered_sample: None,
            next_sample_index: 0,
        }
    }

    fn sample_and_update(&mut self, ctx: &SignalCtx) -> T {
        let sample = self.signal.sample(ctx);
        self.buffered_sample = Some(sample.clone());
        sample
    }

    fn sample(&mut self, ctx: &SignalCtx) -> T {
        if ctx.sample_index < self.next_sample_index {
            if let Some(buffered_sample) = self.buffered_sample.as_ref() {
                buffered_sample.clone()
            } else {
                self.sample_and_update(ctx)
            }
        } else {
            self.next_sample_index = ctx.sample_index + 1;
            self.sample_and_update(ctx)
        }
    }
}

/// A shared caching layer around an implementation of `SignalRaw<T>` which makes sure that the
/// `SignalRaw`'s `sample` method is only called at most once per frame. Its `Clone` implementation
/// is a shallow clone that creates a shared reference to the same underlying signal.
pub struct Signal<T> {
    buffered_signal: Arc<Mutex<SignalUnshared<T>>>,
}

impl<T: Clone + Send + 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            buffered_signal: Arc::clone(&self.buffered_signal),
        }
    }
}

impl<T: Clone + Send + 'static> Signal<T> {
    pub fn new<S: SignalRaw<T> + Send + 'static>(signal: S) -> Self {
        Self {
            buffered_signal: Arc::new(Mutex::new(SignalUnshared::new(signal))),
        }
    }

    pub fn from_fn<F: FnMut(&SignalCtx) -> T + Send + 'static>(f: F) -> Self {
        Self::new(raw_fn(f))
    }

    pub fn sample(&self, ctx: &SignalCtx) -> T {
        self.buffered_signal.lock().unwrap().sample(ctx)
    }

    pub fn map<U: Clone + Send + 'static, F: FnMut(T) -> U + Send + 'static>(
        &self,
        mut f: F,
    ) -> Signal<U> {
        let signal = self.clone();
        Signal::from_fn(move |ctx| f(signal.sample(ctx)))
    }

    pub fn map_ctx<U: Clone + Send + 'static, F: FnMut(T, &SignalCtx) -> U + Send + 'static>(
        &self,
        mut f: F,
    ) -> Signal<U> {
        let signal = self.clone();
        Signal::from_fn(move |ctx| f(signal.sample(ctx), ctx))
    }

    pub fn both<U: Clone + Send + 'static>(&self, other: &Signal<U>) -> Signal<(T, U)> {
        let signal = self.clone();
        let other = other.clone();
        Signal::from_fn(move |ctx| {
            let t = signal.sample(ctx);
            let u = other.sample(ctx);
            (t, u)
        })
    }

    pub fn filter<F: Filter<Input = T> + Send + 'static>(&self, mut filter: F) -> Signal<F::Output>
    where
        F::Output: Clone + Send + 'static,
    {
        self.map_ctx(move |x, ctx| filter.run(x, ctx))
    }

    pub fn debug<F: FnMut(T) + Send + 'static>(&self, mut f: F) -> Self {
        self.map(move |x| {
            f(x.clone());
            x
        })
    }

    /// Force the evaluation of a signal, inoring its value. Use when laziness would otherwise
    /// prevent the evaluation of a signal.
    pub fn force<U, SL: SignalLike<U> + Clone + Send + 'static>(&self, other: &SL) -> Self {
        let other = other.clone();
        let signal = self.clone();
        Signal::from_fn(move |ctx| {
            let _ = other.sample(ctx);
            signal.sample(ctx)
        })
    }
}

impl<T: Clone + std::fmt::Debug + Send + 'static> Signal<T> {
    pub fn debug_print(&self) -> Self {
        self.debug(|x| println!("{:?}", x))
    }
}

pub type Sfreq = Signal<Freq>;
pub type Sf64 = Signal<f64>;
pub type Sf32 = Signal<f32>;
pub type Sbool = Signal<bool>;
pub type Su8 = Signal<u8>;

pub fn const_<T: Copy + Send + 'static>(value: T) -> Signal<T> {
    Signal::from_fn(move |_| value)
}

impl From<Freq> for Sfreq {
    fn from(value: Freq) -> Self {
        const_(value)
    }
}

impl From<f64> for Sf64 {
    fn from(value: f64) -> Self {
        const_(value)
    }
}

impl From<f32> for Sf32 {
    fn from(value: f32) -> Self {
        const_(value)
    }
}

impl From<&f64> for Sf64 {
    fn from(value: &f64) -> Self {
        const_(*value)
    }
}

impl From<&f32> for Sf32 {
    fn from(value: &f32) -> Self {
        const_(*value)
    }
}

impl From<Sf32> for Sf64 {
    fn from(value: Sf32) -> Self {
        value.map(|x| x as f64)
    }
}

impl From<Sf64> for Sf32 {
    fn from(value: Sf64) -> Self {
        value.map(|x| x as f32)
    }
}

impl From<f64> for Sf32 {
    fn from(value: f64) -> Self {
        const_(value as f32)
    }
}

impl From<f32> for Sf64 {
    fn from(value: f32) -> Self {
        const_(value as f64)
    }
}

impl From<&f64> for Sf32 {
    fn from(value: &f64) -> Self {
        const_(*value as f32)
    }
}

impl From<&f32> for Sf64 {
    fn from(value: &f32) -> Self {
        const_(*value as f64)
    }
}

impl Signal<bool> {
    /// Treat `self` as a trigger without performing any gate-to-trigger conversion.
    pub fn to_trigger_raw(&self) -> Trigger {
        Trigger(self.clone())
    }

    pub fn to_trigger_rising_edge(&self) -> Trigger {
        let mut previous = false;
        let signal = self.clone();
        Trigger(Signal::from_fn(move |ctx| {
            let sample = signal.sample(ctx);
            let trigger_sample = sample && !previous;
            previous = sample;
            trigger_sample
        }))
    }

    pub fn to_gate(&self) -> Gate {
        Gate(self.clone())
    }
}

impl Signal<f64> {
    pub fn max(&self, rhs: impl Into<Sf64>) -> Self {
        self.both(&rhs.into()).map(|(x, y)| x.max(y))
    }
    pub fn min(&self, rhs: impl Into<Sf64>) -> Self {
        self.both(&rhs.into()).map(|(x, y)| x.min(y))
    }

    pub fn clamp_non_negative(&self) -> Self {
        self.map(|x| x.max(0.0))
    }

    /// The function f(x) =
    ///   k > 0  => exp(k * (x - a)) - b
    ///   k == 0 => x
    ///   k < 0  => -(ln(x + b) / k) + a
    /// ...where a and b are chosen so that f(0) = 0 and f(1) = 1.
    /// The k parameter controls how sharp the curve is.
    /// The functions when k != 0 are inverses of each other and both approach linearity as k
    /// approaches 0.
    pub fn exp_01(&self, k: impl Into<Sf64>) -> Self {
        let k = k.into();
        self.map_ctx(move |x, ctx| {
            let k = k.sample(ctx);
            let b = 1.0 / k.abs().exp_m1();
            let a = -b.ln() / k.abs();
            if k > 0.0 {
                (k * (x - a)).exp() - b
            } else if k < 0.0 {
                -((x + b).ln() / k) + a
            } else {
                x
            }
        })
    }

    pub fn signed_to_01(&self) -> Self {
        (self + 1.0) / 2.0
    }

    pub fn inv_01(&self) -> Self {
        self.map(|x| 1.0 - x)
    }

    /// Evaluate `by`, and then only evaluate `self` and multiply it by the value of `by` if `by`
    /// is non-zero. Otherwise just return 0.
    pub fn mul_lazy(&self, by: &Self) -> Self {
        let signal = self.clone();
        let by = by.clone();
        Signal::from_fn(move |ctx| {
            let by = by.sample(ctx);
            if by == 0.0 {
                0.0
            } else {
                signal.sample(ctx) * by
            }
        })
    }

    /// If `self` evaluates to a non-zero value then `other` will be evaluated and ignored. If
    /// `self` evaluates to zero then keep evaluating `other` until it too evaluates to zero, then
    /// stop evaluating `other` until `self` becomes non-zero. This is for the situation where you
    /// have an envelope generator used for scaling volume with `mul_lazy` and have a separate
    /// envelope generator used for effects (such as a low-pass filter) with a longer release than
    /// the volume envelope. The use of `mul_lazy` would prevent the effect envelope generator from
    /// completing its release if the volume envelope release completes first. This creates a
    /// problem when the released key is pressed a second time, the effect envelope generator will
    /// be at a non-zero state, so the full effect of its attack will be lost.
    pub fn force_lazy<SL: SignalLike<f64> + Clone + Send + 'static>(&self, other: &SL) -> Self {
        let other = other.clone();
        let signal = self.clone();
        let mut stopped = false;
        Signal::from_fn(move |ctx| {
            let signal_value = signal.sample(ctx);
            if signal_value == 0.0 {
                if !stopped {
                    let other_value = other.sample(ctx);
                    stopped = other_value == 0.0;
                }
            } else {
                let _ = other.sample(ctx);
                stopped = false;
            }
            signal_value
        })
    }
}

impl Signal<Freq> {
    pub fn hz(&self) -> Sf64 {
        self.map(|s| s.hz)
    }
    pub fn s(&self) -> Sf64 {
        self.map(|s| s.s())
    }
}

impl Signal<u8> {
    pub fn midi_index_to_freq_hz_a440(&self) -> Signal<f64> {
        self.map(crate::music::freq_hz_of_midi_index)
    }

    pub fn midi_index_to_freq_a440(&self) -> Signal<Freq> {
        sfreq_hz(self.midi_index_to_freq_hz_a440())
    }
}

#[derive(Clone)]
pub struct Trigger(Sbool);

impl Trigger {
    pub fn never() -> Self {
        Self(const_(false))
    }
    pub fn sample(&self, ctx: &SignalCtx) -> bool {
        self.0.sample(ctx)
    }

    pub fn to_signal(&self) -> Sbool {
        self.0.clone()
    }

    pub fn to_gate(&self) -> Gate {
        self.0.to_gate()
    }

    pub fn and_fn_ctx<F: FnMut(&SignalCtx) -> bool + Send + 'static>(&self, mut f: F) -> Self {
        Self(self.0.map_ctx(move |value, ctx| value && f(ctx)))
    }

    pub fn debug<F: FnMut(bool) + Send + 'static>(&self, f: F) -> Self {
        self.to_signal().debug(f).to_trigger_raw()
    }

    pub fn debug_print(&self) -> Self {
        self.to_signal().debug_print().to_trigger_raw()
    }
}

impl From<Trigger> for Signal<bool> {
    fn from(value: Trigger) -> Self {
        value.to_signal()
    }
}

#[derive(Clone)]
pub struct Gate(Sbool);

impl Gate {
    pub fn never() -> Self {
        Self(const_(false))
    }

    pub fn sample(&self, ctx: &SignalCtx) -> bool {
        self.0.sample(ctx)
    }

    pub fn to_signal(&self) -> Sbool {
        self.0.clone()
    }

    pub fn to_trigger_rising_edge(&self) -> Trigger {
        self.0.to_trigger_rising_edge()
    }

    pub fn debug<F: FnMut(bool) + Send + 'static>(&self, f: F) -> Self {
        self.to_signal().debug(f).to_gate()
    }

    pub fn debug_print(&self) -> Self {
        self.to_signal().debug_print().to_gate()
    }

    pub fn from_fn<F: FnMut(&SignalCtx) -> bool + Send + 'static>(f: F) -> Self {
        Self(Signal::new(raw_fn(f)))
    }
}

impl From<Signal<bool>> for Gate {
    fn from(value: Signal<bool>) -> Self {
        value.to_gate()
    }
}

impl From<Gate> for Signal<bool> {
    fn from(value: Gate) -> Self {
        value.to_signal()
    }
}

pub trait Filter {
    type Input;
    type Output;

    fn run(&mut self, input: Self::Input, ctx: &SignalCtx) -> Self::Output;
}

pub fn sum(signals: impl IntoIterator<Item = Sf64>) -> Sf64 {
    let mut out = const_(0.0);
    for signal in signals {
        out = out + signal;
    }
    out
}

pub fn sum_parallel(signals: impl IntoIterator<Item = Sf64>) -> Sf64 {
    let signals = signals.into_iter().collect::<Vec<_>>();
    Signal::from_fn(move |ctx| {
        use rayon::prelude::*;
        signals
            // add 1 to avoid a small chunk for the remainder
            .par_chunks((signals.len() / ctx.num_threads) + 1)
            .map(|signals| {
                let mut sum = 0.0;
                for signal in signals {
                    sum += signal.sample(ctx);
                }
                sum
            })
            .sum()
    })
}

pub fn mean(signals: impl IntoIterator<Item = Sf64>) -> Sf64 {
    let mut out = const_(0.0);
    let mut count = 0;
    for signal in signals {
        out = out + signal;
        count += 1;
    }
    out / (count as f64)
}

pub trait SignalLike<T> {
    fn sample(&self, ctx: &SignalCtx) -> T;
}

impl<T: Clone + Send + 'static> SignalLike<T> for Signal<T> {
    fn sample(&self, ctx: &SignalCtx) -> T {
        Signal::sample(self, ctx)
    }
}

impl SignalLike<bool> for Trigger {
    fn sample(&self, ctx: &SignalCtx) -> bool {
        Trigger::sample(self, ctx)
    }
}

impl SignalLike<bool> for Gate {
    fn sample(&self, ctx: &SignalCtx) -> bool {
        Gate::sample(self, ctx)
    }
}

impl<T: Clone + Send + 'static> From<&Signal<T>> for Signal<T> {
    fn from(value: &Signal<T>) -> Self {
        value.clone()
    }
}

impl From<&Gate> for Gate {
    fn from(value: &Gate) -> Self {
        value.clone()
    }
}

impl From<&Trigger> for Trigger {
    fn from(value: &Trigger) -> Self {
        value.clone()
    }
}
