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

pub struct ClockedMidiNoteMonophonicLooper {
    pub clock: Trigger,
    pub input_gate: Gate,
    pub input_midi_index: Su8,
    pub clear: Gate,
    pub length: usize,
}

impl ClockedMidiNoteMonophonicLooper {
    pub fn signal(self) -> (Gate, Su8) {
        let Self {
            clock,
            input_gate,
            input_midi_index,
            clear,
            length,
        } = self;
        #[derive(Clone, Copy)]
        struct Entry {
            midi_index: u8,
            key_down: bool,
            key_up: bool,
        }
        let mut sequence = (0..length)
            .map(|_| Entry {
                midi_index: 0,
                key_down: false,
                key_up: false,
            })
            .collect::<Vec<_>>();
        let mut samples_since_last_clock_pulse = 0;
        let mut next_index = 0;
        let mut gate_state = false;
        let mut tap = false;
        let mut output = (false, 0);
        let mut last_period_samples = 0;
        let combined_signal = Signal::from_fn(move |ctx| {
            if tap {
                tap = false;
                output.0 = false;
            }
            samples_since_last_clock_pulse += 1;
            if clock.sample(ctx) {
                if clear.sample(ctx) {
                    let entry = &mut sequence[next_index];
                    entry.key_down = false;
                    entry.key_up = true;
                }
                if input_gate.sample(ctx) {
                    let entry = &mut sequence[next_index];
                    entry.midi_index = input_midi_index.sample(ctx);
                    entry.key_down = true;
                    entry.key_up = false;
                }
                let entry = sequence[next_index];
                if entry.key_down {
                    output = (true, entry.midi_index);
                    if entry.key_up {
                        tap = true;
                    }
                } else {
                    output.0 = false;
                }
                next_index = (next_index + 1) % length;
                last_period_samples = samples_since_last_clock_pulse;
                samples_since_last_clock_pulse = 0;
            }
            let next_gate_state = input_gate.sample(ctx);
            if next_gate_state {
                if !gate_state {
                    // key was just pressed
                    let index_to_update =
                        if samples_since_last_clock_pulse < last_period_samples / 2 {
                            (next_index + length - 1) % length
                        } else {
                            next_index
                        };
                    let entry = &mut sequence[index_to_update];
                    entry.midi_index = input_midi_index.sample(ctx);
                    entry.key_down = true;
                    entry.key_up = false;
                }
                output = (true, input_midi_index.sample(ctx));
            } else {
                if gate_state {
                    // key was just released
                    output = (false, input_midi_index.sample(ctx));
                    let index_to_update =
                        if samples_since_last_clock_pulse < last_period_samples / 2 {
                            (next_index + length - 1) % length
                        } else {
                            next_index
                        };
                    let entry = &mut sequence[index_to_update];
                    entry.key_up = true;
                }
            }
            gate_state = next_gate_state;
            output
        });
        let gate = combined_signal.map(|s| s.0).to_gate();
        let midi_index = combined_signal.map(|s| s.1);
        (gate, midi_index)
    }
}
