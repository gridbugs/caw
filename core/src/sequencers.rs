use crate::signal::{Sf64, Trigger, Triggerable};
use std::{cell::RefCell, rc::Rc};

pub struct SequencedTriggers {
    pub triggers: Vec<Trigger>,
    pub complete: Trigger,
}

pub fn bitwise_pattern_triggers_8(trigger: Trigger, pattern: Vec<u8>) -> SequencedTriggers {
    let i = Rc::new(RefCell::new(0));
    let pattern_signal = trigger.on({
        let i = Rc::clone(&i);
        move || {
            let mut i = i.borrow_mut();
            let out = pattern[*i];
            *i = (*i + 1) % pattern.len();
            out
        }
    });
    let triggers = (0..8)
        .map(|i| {
            pattern_signal
                .map(move |pattern_entry| {
                    if let Some(pattern_entry) = pattern_entry {
                        pattern_entry & (1 << i) != 0
                    } else {
                        false
                    }
                })
                .to_trigger_raw()
        })
        .collect();
    let complete = pattern_signal
        .map({
            let i = Rc::clone(&i);
            move |_| *i.borrow() == 0
        })
        .to_trigger_raw();
    SequencedTriggers { triggers, complete }
}

pub fn drum_loop_8(trigger: Trigger, pattern: Vec<u8>, drums: Vec<Triggerable<f64>>) -> Sf64 {
    let SequencedTriggers { triggers, .. } = bitwise_pattern_triggers_8(trigger, pattern);
    triggers
        .into_iter()
        .zip(drums.into_iter())
        .map(|(trigger, drum)| drum.signal(trigger))
        .sum()
}
