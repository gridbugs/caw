use std::{cell::RefCell, rc::Rc};

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
}

pub type Sf64 = Signal<f64>;
pub type Sf32 = Signal<f32>;
pub type Sbool = Signal<bool>;
pub type Su8 = Signal<u8>;

pub fn const_<T: Clone + 'static>(value: T) -> Signal<T> {
    Signal::from_fn(move |_| value.clone())
}

impl From<f64> for Sf64 {
    fn from(value: f64) -> Self {
        const_(value)
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
}

pub struct Gate(Sbool);

impl Gate {
    pub fn sample(&mut self, ctx: &SignalCtx) -> bool {
        self.0.sample(ctx)
    }

    pub fn to_signal(&self) -> Sbool {
        self.0.clone()
    }
}
