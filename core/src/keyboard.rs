use crate::{
    music::Note,
    signal::{Gate, Sf64, Signal},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub struct VoiceDesc {
    pub note: Signal<Note>,
    pub gate: Gate,
    pub velocity_01: Sf64,
}

#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
    pub note: Note,
    pub pressed: bool,
    pub velocity_01: f64,
}

impl VoiceDesc {
    pub fn monophonic_from_key_events(key_events: Signal<Vec<KeyEvent>>) -> Self {
        #[derive(Clone, Copy, Default)]
        struct HeldKey {
            note: Note,
            velocity_01: f64,
        }

        impl HeldKey {
            fn from_key_event(key_event: &KeyEvent) -> Self {
                Self {
                    note: key_event.note,
                    velocity_01: key_event.velocity_01,
                }
            }
        }

        #[derive(Default)]
        struct State {
            held_keys: Vec<HeldKey>,
            /// The information to use when no key is held
            sticky: HeldKey,
        }

        impl State {
            fn handle_key_event(&mut self, key_event: &KeyEvent) {
                // Remove the held key if it already exists. This assumes there are no duplicate
                // keys in the vector.
                for (i, held_key) in self.held_keys.iter().enumerate() {
                    if held_key.note == key_event.note {
                        self.held_keys.remove(i);
                        break;
                    }
                }
                self.sticky = HeldKey::from_key_event(key_event);
                if key_event.pressed {
                    self.held_keys.push(self.sticky);
                }
            }
            fn last(&self) -> Option<&HeldKey> {
                self.held_keys.last()
            }
        }

        let state = Rc::new(RefCell::new(State::default()));
        let update_state = key_events.map({
            let state = Rc::clone(&state);
            move |key_events_this_tick| {
                for key_event in &key_events_this_tick {
                    state.borrow_mut().handle_key_event(key_event);
                }
            }
        });
        let note = update_state.map({
            let state = Rc::clone(&state);
            move |()| {
                let state = state.borrow();
                if let Some(last) = state.last() {
                    last.note
                } else {
                    state.sticky.note
                }
            }
        });
        let gate = update_state
            .map({
                let state = Rc::clone(&state);
                move |()| state.borrow().last().is_some()
            })
            .to_gate();
        let velocity_01 = update_state.map({
            let state = Rc::clone(&state);
            move |()| {
                let state = state.borrow();
                if let Some(last) = state.last() {
                    last.velocity_01
                } else {
                    state.sticky.velocity_01
                }
            }
        });
        Self {
            note,
            gate,
            velocity_01,
        }
    }
}

impl Signal<Vec<KeyEvent>> {
    pub fn voice_desc_monophonic(self) -> VoiceDesc {
        VoiceDesc::monophonic_from_key_events(self)
    }
}
