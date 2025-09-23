use caw_persistent::PersistentData;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

impl PersistentData for WindowPosition {
    const NAME: &'static str = "window_position";
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl PersistentData for WindowSize {
    const NAME: &'static str = "window_size";
}
