use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
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
    let rng = RefCell::new(StdRng::from_entropy());
    Signal::from_fn(move |_ctx| (rng.borrow_mut().gen::<f64>() * 2.0) - 1.0)
}

pub fn noise_01() -> Sf64 {
    let rng = RefCell::new(StdRng::from_entropy());
    Signal::from_fn(move |_ctx| (rng.borrow_mut().gen::<f64>()))
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
    fn sample(&self, ctx: &SignalCtx) -> T;
}

pub struct SignalRawFn<T, F: Fn(&SignalCtx) -> T>(F);

impl<T, F: Fn(&SignalCtx) -> T> SignalRaw<T> for SignalRawFn<T, F> {
    fn sample(&self, ctx: &SignalCtx) -> T {
        (self.0)(ctx)
    }
}

/// Caching layer around a `SignalRaw` which makes sure that its `sample` method is only called at
/// most once per frame. It is "unshared" in the sense that it owns the `SignalRaw` instance that
/// it is wrapping.
struct SignalUnshared<T: Clone + Default, S: SignalRaw<T>> {
    signal: S,
    buffered_sample: RefCell<T>,
    next_sample_index: Cell<u64>,
}

impl<T: Clone + Default, S: SignalRaw<T>> SignalUnshared<T, S> {
    fn new(signal: S) -> Self {
        Self {
            signal,
            buffered_sample: RefCell::new(T::default()),
            next_sample_index: Cell::new(0),
        }
    }

    fn sample(&self, ctx: &SignalCtx) -> T {
        if ctx.sample_index < self.next_sample_index.get() {
            self.buffered_sample.borrow().clone()
        } else {
            self.next_sample_index.set(ctx.sample_index + 1);
            let sample = self.signal.sample(ctx);
            *self.buffered_sample.borrow_mut() = sample.clone();
            sample
        }
    }
}

impl<T: Clone + Default, S: SignalRaw<T>> SignalRaw<T> for SignalUnshared<T, S> {
    fn sample(&self, ctx: &SignalCtx) -> T {
        SignalUnshared::sample(self, ctx)
    }
}

/// A shared caching layer around an implementation of `SignalRaw<T>` which makes sure that the
/// `SignalRaw`'s `sample` method is only called at most once per frame. Its `Clone` implementation
/// is a shallow clone that creates a shared reference to the same underlying signal.
pub struct Signal<T>(Rc<dyn SignalRaw<T>>);

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T: Clone + Default + 'static> Signal<T> {
    pub fn new<S: SignalRaw<T> + 'static>(signal: S) -> Self {
        Self(Rc::new(SignalUnshared::new(signal)))
    }

    pub fn from_fn<F: Fn(&SignalCtx) -> T + 'static>(f: F) -> Self {
        Self::new(SignalRawFn(f))
    }

    pub fn from_fn_mut<F: FnMut(&SignalCtx) -> T + 'static>(f: F) -> Self {
        let cell = RefCell::new(f);
        Self::from_fn(move |ctx| (cell.borrow_mut())(ctx))
    }

    pub fn sample(&self, ctx: &SignalCtx) -> T {
        self.0.sample(ctx)
    }

    pub fn map<U: Clone + Default + 'static, F: Fn(T) -> U + 'static>(&self, f: F) -> Signal<U> {
        let signal = self.clone();
        Signal::from_fn(move |ctx| f(signal.sample(ctx)))
    }

    pub fn map_ctx<U: Clone + Default + 'static, F: Fn(T, &SignalCtx) -> U + 'static>(
        &self,
        f: F,
    ) -> Signal<U> {
        let signal = self.clone();
        Signal::from_fn(move |ctx| f(signal.sample(ctx), ctx))
    }

    pub fn zip<U: Clone + Default + 'static>(&self, other: &Signal<U>) -> Signal<(T, U)> {
        let signal = self.clone();
        let other = other.clone();
        Signal::from_fn(move |ctx| {
            let t = signal.sample(ctx);
            let u = other.sample(ctx);
            (t, u)
        })
    }

    pub fn filter<F: Filter<Input = T> + 'static>(&self, filter: F) -> Signal<F::Output>
    where
        F::Output: Clone + Default + 'static,
    {
        self.map_ctx(move |x, ctx| filter.run(x, ctx))
    }

    pub fn debug<F: Fn(T) + 'static>(&self, f: F) -> Self {
        self.map(move |x| {
            f(x.clone());
            x
        })
    }

    /// Force the evaluation of a signal, inoring its value. Use when laziness would otherwise
    /// prevent the evaluation of a signal.
    pub fn force<U, SL: SignalLike<U> + Clone + 'static>(&self, other: &SL) -> Self {
        let other = other.clone();
        let signal = self.clone();
        Signal::from_fn(move |ctx| {
            let _ = other.sample(ctx);
            signal.sample(ctx)
        })
    }

    pub fn with<F: FnOnce(Self) -> Self>(&self, f: F) -> Self {
        f(self.clone())
    }
}

impl<T: Clone + Default + std::fmt::Debug + 'static> Signal<T> {
    pub fn debug_print(&self) -> Self {
        self.debug(|x| println!("{:?}", x))
    }
}

pub type Sfreq = Signal<Freq>;
pub type Sf64 = Signal<f64>;
pub type Sf32 = Signal<f32>;
pub type Sbool = Signal<bool>;
pub type Su8 = Signal<u8>;

pub fn const_<T: Clone + Default + 'static>(value: T) -> Signal<T> {
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
impl From<u32> for Signal<u32> {
    fn from(value: u32) -> Self {
        const_(value)
    }
}

impl Signal<bool> {
    /// Treat `self` as a trigger without performing any gate-to-trigger conversion.
    pub fn to_trigger_raw(&self) -> Trigger {
        Trigger(self.clone())
    }

    pub fn to_trigger_rising_edge(&self) -> Trigger {
        let previous = Cell::new(false);
        let signal = self.clone();
        Trigger(Signal::from_fn(move |ctx| {
            let sample = signal.sample(ctx);
            let trigger_sample = sample && !previous.get();
            previous.set(sample);
            trigger_sample
        }))
    }

    pub fn to_gate(&self) -> Gate {
        Gate(self.clone())
    }
}

impl Signal<f64> {
    pub fn max(&self, rhs: impl Into<Sf64>) -> Self {
        self.zip(&rhs.into()).map(|(x, y)| x.max(y))
    }
    pub fn min(&self, rhs: impl Into<Sf64>) -> Self {
        self.zip(&rhs.into()).map(|(x, y)| x.min(y))
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
    /// The functions when k != 0 are inverses of each other and zip approach linearity as k
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

    pub fn lazy_zero(&self, control: &Self) -> Self {
        let signal = self.clone();
        let control = control.clone();
        Signal::from_fn(move |ctx| {
            let control = control.sample(ctx);
            if control == 0.0 {
                0.0
            } else {
                signal.sample(ctx)
            }
        })
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
    pub fn force_lazy<SL: SignalLike<f64> + Clone + 'static>(&self, other: &SL) -> Self {
        let other = other.clone();
        let signal = self.clone();
        let stopped = Cell::new(false);
        Signal::from_fn(move |ctx| {
            let signal_value = signal.sample(ctx);
            if signal_value == 0.0 {
                if !stopped.get() {
                    let other_value = other.sample(ctx);
                    stopped.set(other_value == 0.0);
                }
            } else {
                let _ = other.sample(ctx);
                stopped.set(false);
            }
            signal_value
        })
    }

    pub fn mix<F: FnOnce(Self) -> Self>(&self, f: F) -> Self {
        self.with(move |dry| dry.clone() + f(dry))
    }
}

impl Signal<Freq> {
    pub fn hz(&self) -> Sf64 {
        self.map(|s| s.hz)
    }
    pub fn s(&self) -> Sf64 {
        self.map(|s| s.s())
    }
    pub fn sample_hz(&self, ctx: &SignalCtx) -> f64 {
        self.sample(ctx).hz
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

    pub fn any(triggers: impl IntoIterator<Item = Trigger> + 'static) -> Self {
        let triggers = triggers.into_iter().collect::<Vec<_>>();
        Signal::from_fn(move |ctx| {
            for trigger in &triggers {
                if trigger.sample(ctx) {
                    return true;
                }
            }
            false
        })
        .to_trigger_raw()
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

    pub fn to_gate_with_duration_s(&self, duration_s: impl Into<Sf64>) -> Gate {
        let trigger = self.clone();
        let duration_s = duration_s.into();
        let rem_samples = Cell::new(0.0);
        Gate::from_fn(move |ctx| {
            if trigger.sample(ctx) {
                rem_samples.set(ctx.sample_rate_hz * duration_s.sample(ctx));
            }
            let rem_samples_val = rem_samples.get();
            if rem_samples_val <= 0.0 {
                false
            } else {
                rem_samples.set(rem_samples_val - 1.0);
                true
            }
        })
    }

    pub fn on<T: Clone + 'static, F: Fn() -> T + 'static>(&self, f: F) -> Signal<Option<T>> {
        self.0.map(move |x| if x { Some(f()) } else { None })
    }

    pub fn on_ctx<T: Clone + 'static, F: Fn(&SignalCtx) -> T + 'static>(
        &self,
        f: F,
    ) -> Signal<Option<T>> {
        self.0
            .map_ctx(move |x, ctx| if x { Some(f(ctx)) } else { None })
    }

    pub fn and_fn<F: Fn() -> bool + 'static>(&self, f: F) -> Self {
        Self(self.0.map(move |value| value && f()))
    }

    pub fn and_fn_ctx<F: Fn(&SignalCtx) -> bool + 'static>(&self, f: F) -> Self {
        Self(self.0.map_ctx(move |value, ctx| value && f(ctx)))
    }

    pub fn debug<F: Fn(bool) + 'static>(&self, f: F) -> Self {
        self.to_signal().debug(f).to_trigger_raw()
    }

    pub fn debug_print(&self) -> Self {
        self.to_signal().debug_print().to_trigger_raw()
    }

    pub fn divide(&self, by: impl Into<Signal<u32>> + 'static) -> Self {
        let by = by.into();
        let count = Cell::new(0);
        self.and_fn_ctx(move |ctx| {
            let count_val = count.get();
            count.set((count_val + 1) % by.sample(ctx).max(1));
            count_val == 0
        })
    }

    pub fn random_skip(&self, probability_01: impl Into<Signal<f64>>) -> Self {
        let probability_01 = probability_01.into();
        let noise = noise_01();
        self.and_fn_ctx(move |ctx| noise.sample(ctx) > probability_01.sample(ctx))
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

    pub fn debug<F: Fn(bool) + 'static>(&self, f: F) -> Self {
        self.to_signal().debug(f).to_gate()
    }

    pub fn debug_print(&self) -> Self {
        self.to_signal().debug_print().to_gate()
    }

    pub fn from_fn<F: Fn(&SignalCtx) -> bool + 'static>(f: F) -> Self {
        Self(Signal::new(SignalRawFn(f)))
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

    fn run(&self, input: Self::Input, ctx: &SignalCtx) -> Self::Output;
}

pub fn sum(signals: impl IntoIterator<Item = Sf64>) -> Sf64 {
    let mut out = const_(0.0);
    for signal in signals {
        out = out + signal;
    }
    out
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

impl<T: Clone + Default + 'static> SignalLike<T> for Signal<T> {
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

impl<T: Clone + 'static> From<&Signal<T>> for Signal<T> {
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
