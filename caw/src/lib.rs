pub use caw_builder_proc_macros as builder_proc_macros;
pub use caw_computer_keyboard as computer_keyboard;
pub use caw_core as core;
pub use caw_keyboard as keyboard;
pub use caw_modules as modules;
pub use caw_patches as patches;
pub use caw_utils as utils;

#[cfg(feature = "caw_player")]
pub use caw_player as player;

#[cfg(feature = "caw_midi")]
pub use caw_midi as midi;

#[cfg(feature = "caw_midi_live")]
pub use caw_midi_live as midi_live;

#[cfg(feature = "caw_midi_file")]
pub use caw_midi_file as midi_file;

#[cfg(feature = "caw_midi_serial")]
pub use caw_midi_serial as midi_serial;

#[cfg(feature = "caw_interactive")]
pub use caw_interactive as interactive;

#[cfg(feature = "caw_audio_file")]
pub use caw_audio_file as audio_file;

#[cfg(feature = "caw_live")]
pub use caw_live as live;

#[cfg(feature = "caw_midi_udp_widgets_app_lib")]
pub use caw_midi_udp_widgets_app_lib as midi_udp_widgets_app_lib;

#[cfg(feature = "caw_viz_udp_app_lib")]
pub use caw_viz_udp_app_lib as viz_udp_app_lib;

pub mod prelude {
    pub use super::builder_proc_macros::*;
    pub use super::computer_keyboard::*;
    pub use super::core::*;
    pub use super::keyboard::*;
    pub use super::modules::*;
    pub use super::patches::*;
    pub use super::utils::*;

    #[cfg(feature = "caw_player")]
    pub use super::player::*;

    #[cfg(feature = "caw_midi")]
    pub use super::midi::*;

    #[cfg(feature = "caw_midi_live")]
    pub use super::midi_live::*;

    #[cfg(feature = "caw_midi_file")]
    pub use super::midi_file::*;

    #[cfg(feature = "caw_midi_serial")]
    pub use super::midi_serial::*;

    #[cfg(feature = "caw_interactive")]
    pub use super::interactive::*;

    #[cfg(feature = "caw_audio_file")]
    pub use super::audio_file::*;

    #[cfg(feature = "caw_live")]
    pub use super::live::*;

    #[cfg(feature = "caw_midi_udp_widgets_app_lib")]
    pub use super::midi_udp_widgets_app_lib::*;

    #[cfg(feature = "caw_viz_udp_app_lib")]
    pub use super::viz_udp_app_lib::*;
}
