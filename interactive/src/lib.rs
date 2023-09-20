pub use ibis::{
    clock, envelope, filters, midi, music, oscillator, sample_player, signal, signal_player,
};
pub mod input;
pub mod window;
pub mod prelude {
    pub use crate::{
        input::{Input, Key},
        window::{Rgb24, Window},
    };
    pub use ibis::prelude::*;
}
