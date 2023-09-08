use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Freq {
    hz: f64,
}

impl Freq {
    pub fn from_hz(hz: f64) -> Self {
        Self { hz }
    }

    pub fn from_s(s: f64) -> Self {
        Self::from_hz(1.0 / s)
    }

    pub fn hz(&self) -> f64 {
        self.hz
    }

    pub fn s(&self) -> f64 {
        self.hz() / 1.0
    }
}

pub fn freq_hz(hz: f64) -> Freq {
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

/// Information about the current state of playback that will be passed to every signal each frame
pub struct SignalCtx {
    pub sample_index: u64,
    pub sample_rate_hz: f64,
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
    signal: Box<dyn SignalRaw<T>>,
    buffered_sample: Option<T>,
    next_sample_index: u64,
}

impl<T: Clone> SignalUnshared<T> {
    fn new<S: SignalRaw<T> + 'static>(signal: S) -> Self {
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
pub struct Signal<T>(Rc<RefCell<SignalUnshared<T>>>);

impl<T: Clone + 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T: Clone + 'static> Signal<T> {
    pub fn new<S: SignalRaw<T> + 'static>(signal: S) -> Self {
        Self(Rc::new(RefCell::new(SignalUnshared::new(signal))))
    }

    pub fn from_fn<F: FnMut(&SignalCtx) -> T + 'static>(f: F) -> Self {
        Self::new(raw_fn(f))
    }

    pub fn sample(&mut self, ctx: &SignalCtx) -> T {
        self.0.borrow_mut().sample(ctx)
    }

    pub fn map<U: Clone + 'static, F: FnMut(T) -> U + 'static>(&self, mut f: F) -> Signal<U> {
        let mut signal = self.clone();
        Signal::from_fn(move |ctx| f(signal.sample(ctx)))
    }

    pub fn map_ctx<U: Clone + 'static, F: FnMut(T, &SignalCtx) -> U + 'static>(
        &self,
        mut f: F,
    ) -> Signal<U> {
        let mut signal = self.clone();
        Signal::from_fn(move |ctx| f(signal.sample(ctx), ctx))
    }

    pub fn both<U: Clone + 'static>(&self, other: &Signal<U>) -> Signal<(T, U)> {
        let mut signal = self.clone();
        let mut other = other.clone();
        Signal::from_fn(move |ctx| {
            let t = signal.sample(ctx);
            let u = other.sample(ctx);
            (t, u)
        })
    }

    pub fn filter<F: Filter<Input = T> + 'static>(&self, mut filter: F) -> Signal<F::Output>
    where
        F::Output: Clone + 'static,
    {
        self.map_ctx(move |x, ctx| filter.run(x, ctx))
    }

    pub fn debug<F: FnMut(T) + 'static>(&self, mut f: F) -> Self {
        self.map(move |x| {
            f(x.clone());
            x
        })
    }

    /// Force the evaluation of a signal, inoring its value. Use when laziness would otherwise
    /// prevent the evaluation of a signal. When using `mul_lazy` with an amp envelope that is
    /// different from (say) a filter envelope, the filter envelope will need to be forced so that
    /// the internal state of the envelope generator is updated even when the amp envelope goes to
    /// zero. This doesn't defeat the point of `mul_lazy` as it still avoids evaluating filters and
    /// oscillators which are the most expensive parts of most synth patches.
    pub fn force<U, SL: SignalLike<U> + Clone + 'static>(&self, other: &SL) -> Self {
        let mut other = other.clone();
        let mut signal = self.clone();
        Signal::from_fn(move |ctx| {
            let _ = other.sample(ctx);
            signal.sample(ctx)
        })
    }
}

impl<T: Clone + std::fmt::Debug + 'static> Signal<T> {
    pub fn debug_print(&self) -> Self {
        self.debug(|x| println!("{:?}", x))
    }
}

pub type Sfreq = Signal<Freq>;
pub type Sf64 = Signal<f64>;
pub type Sf32 = Signal<f32>;
pub type Sbool = Signal<bool>;
pub type Su8 = Signal<u8>;

pub fn const_<T: Clone + 'static>(value: T) -> Signal<T> {
    Signal::from_fn(move |_| value.clone())
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

impl Signal<bool> {
    /// Treat `self` as a trigger without performing any gate-to-trigger conversion.
    pub fn to_trigger_raw(&self) -> Trigger {
        Trigger(self.clone())
    }

    pub fn to_trigger_rising_edge(&self) -> Trigger {
        let mut previous = false;
        let mut signal = self.clone();
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
    /// The function f(x) = exp(k * (x - a)) - b
    /// ...where a and b are chosen so that f(0) = 0 and f(1) = 1.
    /// The k parameter controls how sharp the curve is.
    /// It approaches a linear function as k approaches 0.
    /// k = 0 is special cased as a linear function for convenience.
    pub fn exp_01(&self, k: f64) -> Self {
        if k == 0.0 {
            self.clone()
        } else {
            let b = 1.0 / k.exp_m1();
            let a = -b.ln() / k;
            self.map(move |x| (k * (x - a)).exp() - b)
        }
    }

    pub fn signed_to_01(&self) -> Self {
        (self + 1.0) / 2.0
    }

    /// Evaluate `by`, and then only evaluate `self` and multiply it by the value of `by` if `by`
    /// is non-zero. Otherwise just return 0.
    pub fn mul_lazy(&self, by: &Self) -> Self {
        let mut signal = self.clone();
        let mut by = by.clone();
        Signal::from_fn(move |ctx| {
            let by = by.sample(ctx);
            if by == 0.0 {
                0.0
            } else {
                signal.sample(ctx) * by
            }
        })
    }
}

#[derive(Clone)]
pub struct Trigger(Sbool);

impl Trigger {
    pub fn never() -> Self {
        Self(const_(false))
    }
    pub fn sample(&mut self, ctx: &SignalCtx) -> bool {
        self.0.sample(ctx)
    }

    pub fn to_signal(&self) -> Sbool {
        self.0.clone()
    }

    pub fn to_gate(&self) -> Gate {
        self.0.to_gate()
    }

    pub fn debug<F: FnMut(bool) + 'static>(&self, f: F) -> Self {
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

    pub fn sample(&mut self, ctx: &SignalCtx) -> bool {
        self.0.sample(ctx)
    }

    pub fn to_signal(&self) -> Sbool {
        self.0.clone()
    }

    pub fn to_trigger_rising_edge(&self) -> Trigger {
        self.0.to_trigger_rising_edge()
    }

    pub fn debug<F: FnMut(bool) + 'static>(&self, f: F) -> Self {
        self.to_signal().debug(f).to_gate()
    }

    pub fn debug_print(&self) -> Self {
        self.to_signal().debug_print().to_gate()
    }

    pub fn from_fn<F: FnMut(&SignalCtx) -> bool + 'static>(f: F) -> Self {
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

pub trait SignalLike<T> {
    fn sample(&mut self, ctx: &SignalCtx) -> T;
}

impl<T: Clone + 'static> SignalLike<T> for Signal<T> {
    fn sample(&mut self, ctx: &SignalCtx) -> T {
        Signal::sample(self, ctx)
    }
}

impl SignalLike<bool> for Trigger {
    fn sample(&mut self, ctx: &SignalCtx) -> bool {
        Trigger::sample(self, ctx)
    }
}

impl SignalLike<bool> for Gate {
    fn sample(&mut self, ctx: &SignalCtx) -> bool {
        Gate::sample(self, ctx)
    }
}
