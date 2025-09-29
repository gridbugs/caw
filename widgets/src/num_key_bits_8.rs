use crate::window::{TitlePosition, Window};
use anyhow::anyhow;
use caw_computer_keyboard::Key;
use caw_keyboard::Note;
use caw_window_utils::font::{Font, load_font};
use midly::{MidiMessage, num::u7};
use sdl2::{keyboard::Scancode, pixels::Color, rect::Rect};
use std::{collections::HashMap, time::Instant};

const WIDTH_PX: u32 = 128;
const HEIGHT_PX: u32 = 128;

pub struct NumKeyBits8 {
    window: Window,
    font: Font<'static, 'static>,
}

const SPACEBAR_CONTROLLER: u7 = u7::from_int_lossy(0);

impl NumKeyBits8 {}
