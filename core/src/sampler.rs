use crate::signal::{Sf64, Signal, Trigger};
use std::cell::Cell;

#[derive(Clone)]
pub struct Sample {
    samples: Vec<f64>,
}

impl Sample {
    pub fn new(samples: Vec<f64>) -> Self {
        Self { samples }
    }
}

#[derive(Clone)]
pub struct Sampler {
    sample: Sample,
    index: usize,
    play: Trigger,
}

impl Sampler {
    pub fn new(sample: &Sample, play: Trigger) -> Self {
        let index = sample.samples.len(); // this prevents the sample from immediately playing
        Self {
            sample: sample.clone(),
            index,
            play,
        }
    }

    pub fn signal(self) -> Sf64 {
        let Self {
            sample,
            index,
            play,
        } = self;
        let index = Cell::new(index);
        Signal::from_fn(move |ctx| {
            if play.sample(ctx) {
                index.set(0);
            }
            let index_value = index.get();
            if index_value < sample.samples.len() {
                let output = sample.samples[index_value];
                index.set(index_value + 1);
                output
            } else {
                0.0
            }
        })
    }
}
