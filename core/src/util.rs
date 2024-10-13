use crate::signal::{Sf64, Signal, Trigger};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub fn bitwise_trigger_router_64(
    input: Trigger,
    selection: Signal<u64>,
) -> Vec<Trigger> {
    (0..64)
        .map({
            |i| {
                let selection = selection.clone();
                input.and_fn_ctx(move |ctx| {
                    selection.sample(ctx) & (1 << i) != 0
                })
            }
        })
        .collect()
}

pub fn weighted_random_choice<T: Copy + Default + 'static>(
    choices: Vec<(T, Sf64)>,
) -> Signal<T> {
    let rng = RefCell::new(StdRng::from_entropy());
    Signal::from_fn(move |ctx| {
        let mut total = 0.0;
        for (_, weight) in &choices {
            total += weight.sample(ctx);
        }
        let mut choice_index = rng.borrow_mut().gen::<f64>() * total;
        for (choice, weight) in &choices {
            let weight = weight.sample(ctx);
            if choice_index < weight {
                return *choice;
            }
            choice_index -= weight;
        }
        unreachable!()
    })
}

pub fn generic_sample_and_hold<T: Copy + Default + 'static>(
    signal: Signal<T>,
    sample_trigger: Trigger,
) -> Signal<T> {
    let sample = Cell::new(T::default());
    Signal::from_fn(move |ctx| {
        if sample_trigger.sample(ctx) {
            sample.set(signal.sample(ctx));
        }
        sample.get()
    })
}

pub fn with_fix<
    T: Copy + Default + 'static,
    U: Copy + Default + 'static,
    F: FnOnce(Signal<T>) -> (Signal<T>, Signal<U>),
>(
    f: F,
) -> Signal<U> {
    let signal_cell: Rc<RefCell<Option<Signal<T>>>> =
        Rc::new(RefCell::new(None));
    let signal = Signal::from_fn({
        let signal_cell = Rc::clone(&signal_cell);
        move |ctx| signal_cell.borrow().as_ref().unwrap().sample(ctx)
    });
    let (new_signal, output) = f(signal);
    *signal_cell.borrow_mut() = Some(new_signal);
    output
}

pub fn trigger_split_cycle(trigger: Trigger, n: usize) -> Vec<Trigger> {
    let count = Rc::new(RefCell::new(0));
    (0..n)
        .map(move |i| {
            let count = Rc::clone(&count);
            trigger.and_fn(move || {
                let mut count = count.borrow_mut();
                let output = *count == i;
                *count = (*count + 1) % n;
                output
            })
        })
        .collect()
}
