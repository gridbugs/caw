pub use currawong_core::{clock, envelope, filters, music, oscillator, signal};
pub mod midi;
pub mod sample_player;
pub mod signal_player;
pub mod prelude {
    pub use crate::midi::{MidiFile, MidiLive, MidiLiveSerial};
    pub use currawong_core::prelude::*;
}
