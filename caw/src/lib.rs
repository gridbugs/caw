pub use caw_core::{clock, envelope, filters, music, oscillator, signal};
pub mod microphone;
#[cfg(feature = "midi")]
pub mod midi;
pub mod sample;
pub mod sample_player;
pub mod signal_player;
pub mod prelude {
    pub use crate::microphone::microphone;
    #[cfg(feature = "midi")]
    pub use crate::midi::{MidiFile, MidiLive, MidiLiveSerial};
    pub use crate::sample::read_wav;
    pub use crate::signal_player::SignalPlayer;
    pub use caw_core::prelude::*;
}
