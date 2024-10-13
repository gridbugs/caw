use crate::signal::*;
use std::cell::RefCell;

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
        struct State {
            sequence: Vec<bool>,
            samples_since_last_clock_pulse: usize,
            samples_since_last_add: usize,
            next_index: usize,
        }
        let state = RefCell::new(State {
            sequence: (0..length).map(|_| false).collect::<Vec<_>>(),
            samples_since_last_clock_pulse: 0,
            samples_since_last_add: 0,
            next_index: 0,
        });
        let add_trigger = add.to_trigger_rising_edge();
        Signal::from_fn(move |ctx| {
            let mut state = state.borrow_mut();
            let add_this_sample = add_trigger.sample(ctx);
            let mut output = add_this_sample;
            if add_this_sample {
                state.samples_since_last_add = 0;
            } else {
                state.samples_since_last_add += 1;
            }
            if clock.sample(ctx) {
                let next_index = state.next_index;
                if remove.sample(ctx) {
                    state.sequence[next_index] = false;
                }
                // Set the output before updating the sequence. The sound plays when a key is
                // pressed, and on the clock pulse imediately after we don't want to play the sound
                // a second time. Setting the output here prevents this.
                output = state.sequence[next_index];
                if state.samples_since_last_add
                    < state.samples_since_last_clock_pulse / 2
                {
                    state.sequence[next_index] = true;
                } else {
                    if state.samples_since_last_add
                        < state.samples_since_last_clock_pulse
                    {
                        state.sequence[(next_index + length - 1) % length] =
                            true;
                    }
                    if add.sample(ctx) {
                        state.sequence[next_index] = true;
                        // Explicitly set the output here. When holding a button we fill the
                        // sequence on each clock tick and also play the sound.
                        output = true;
                    }
                }
                state.samples_since_last_clock_pulse = 0;
                state.next_index = (state.next_index + 1) % length;
            } else {
                state.samples_since_last_clock_pulse += 1;
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
        struct State {
            sequence: Vec<Entry>,
            samples_since_last_clock_pulse: usize,
            next_index: usize,
            gate_state: bool,
            tap: bool,
            output: (bool, u8),
            last_period_samples: usize,
        }
        let state = RefCell::new(State {
            sequence: (0..length)
                .map(|_| Entry {
                    midi_index: 0,
                    key_down: false,
                    key_up: false,
                })
                .collect::<Vec<_>>(),
            samples_since_last_clock_pulse: 0,
            next_index: 0,
            gate_state: false,
            tap: false,
            output: (false, 0),
            last_period_samples: 0,
        });
        let combined_signal = Signal::from_fn(move |ctx| {
            let mut state = state.borrow_mut();
            if state.tap {
                state.tap = false;
                state.output.0 = false;
            }
            state.samples_since_last_clock_pulse += 1;
            if clock.sample(ctx) {
                let next_index = state.next_index;
                if clear.sample(ctx) {
                    let entry = &mut state.sequence[next_index];
                    entry.key_down = false;
                    entry.key_up = true;
                }
                if input_gate.sample(ctx) {
                    let entry = &mut state.sequence[next_index];
                    entry.midi_index = input_midi_index.sample(ctx);
                    entry.key_down = true;
                    entry.key_up = false;
                }
                let entry = state.sequence[next_index];
                if entry.key_down {
                    state.output = (true, entry.midi_index);
                    if entry.key_up {
                        state.tap = true;
                    }
                } else {
                    state.output.0 = false;
                }
                state.next_index = (next_index + 1) % length;
                state.last_period_samples =
                    state.samples_since_last_clock_pulse;
                state.samples_since_last_clock_pulse = 0;
            }
            let next_gate_state = input_gate.sample(ctx);
            if next_gate_state {
                if !state.gate_state {
                    // key was just pressed
                    let index_to_update = if state
                        .samples_since_last_clock_pulse
                        < state.last_period_samples / 2
                    {
                        (state.next_index + length - 1) % length
                    } else {
                        state.next_index
                    };
                    let entry = &mut state.sequence[index_to_update];
                    entry.midi_index = input_midi_index.sample(ctx);
                    entry.key_down = true;
                    entry.key_up = false;
                }
                state.output = (true, input_midi_index.sample(ctx));
            } else {
                if state.gate_state {
                    // key was just released
                    state.output = (false, input_midi_index.sample(ctx));
                    let index_to_update = if state
                        .samples_since_last_clock_pulse
                        < state.last_period_samples / 2
                    {
                        (state.next_index + length - 1) % length
                    } else {
                        state.next_index
                    };
                    let entry = &mut state.sequence[index_to_update];
                    entry.key_up = true;
                }
            }
            state.gate_state = next_gate_state;
            state.output
        });
        let gate = combined_signal.map(|s| s.0).to_gate();
        let midi_index = combined_signal.map(|s| s.1);
        (gate, midi_index)
    }
}
