pub use crate::signal::*;

pub struct ClockedTriggerLooper {
    pub clock: Trigger,
    pub add: Gate,
    pub remove: Gate,
    pub length: usize,
}

impl ClockedTriggerLooper {
    pub fn signal(self) -> Sbool {
        let Self {
            clock,
            add,
            remove,
            length,
        } = self;
        let mut sequence = (0..length).map(|_| false).collect::<Vec<_>>();
        let mut samples_since_last_clock_pulse = 0;
        let mut samples_since_last_add = 0;
        let mut next_index = 0;
        let add_trigger = add.to_trigger_rising_edge();
        Signal::from_fn(move |ctx| {
            let add_this_sample = add_trigger.sample(ctx);
            let mut output = add_this_sample;
            if add_this_sample {
                samples_since_last_add = 0;
            } else {
                samples_since_last_add += 1;
            }
            if clock.sample(ctx) {
                if remove.sample(ctx) {
                    sequence[next_index] = false;
                }
                // this is set here to prevent the drum sounding twice the first time it's pressed
                output = sequence[next_index];
                if samples_since_last_add < samples_since_last_clock_pulse / 2 {
                    sequence[next_index] = true;
                } else if samples_since_last_add < samples_since_last_clock_pulse {
                    sequence[(next_index + length - 1) % length] = true;
                } else if add.sample(ctx) {
                    sequence[next_index] = true;
                    output = true;
                }
                samples_since_last_clock_pulse = 0;
                next_index = (next_index + 1) % length;
            } else {
                samples_since_last_clock_pulse += 1;
            }
            output
        })
    }

    pub fn trigger(self) -> Trigger {
        self.signal().to_trigger_raw()
    }
}
