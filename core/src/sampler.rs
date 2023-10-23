use crate::signal::{Sf64, Signal, Trigger};

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
            mut index,
            play,
        } = self;
        Signal::from_fn(move |ctx| {
            if play.sample(ctx) {
                index = 0;
            }
            if index < sample.samples.len() {
                let output = sample.samples[index];
                index += 1;
                output
            } else {
                0.0
            }
        })
    }
}
