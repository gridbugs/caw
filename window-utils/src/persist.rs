use caw_persist::PersistData;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

impl PersistData for WindowPosition {
    const NAME: &'static str = "window_position";
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl PersistData for WindowSize {
    const NAME: &'static str = "window_size";
}
