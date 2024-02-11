pub use currawong::{
    clock, envelope, filters, music, oscillator, sample_player, signal, signal_player,
};

#[cfg(feature = "midi")]
pub use currawong::midi;

pub mod input;
pub mod templates;
pub mod window;
pub mod prelude {
    pub use crate::{
        input::{Input, Key},
        templates::{keyboard_key_events, note_by_key_sequence, opinionated_key_events},
        window::{Rgb24, Window},
    };
    pub use currawong::prelude::*;
}
