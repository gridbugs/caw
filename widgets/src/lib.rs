//! A library of graphical widgets. This library doesn't know about signals or UDP. It just handles
//! creating interactive widgets and getting data from the widgets. The `midi_udp_widgets_app`
//! crate is an executable that uses this library to create widgets and sends data from those
//! widgets over UDP to a synthesizer.

mod button;
mod computer_keyboard;
mod knob;
mod window;
mod xy;

pub use button::*;
pub use computer_keyboard::*;
pub use knob::*;
pub use xy::*;
