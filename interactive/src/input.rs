pub use caw_computer_keyboard::{Key, Keyboard as KeyboardGeneric};
use caw_core::{Sig, SigT, Variable, variable};
use sdl2::{keyboard::Scancode, mouse::MouseButton as SdlMouseButton};

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Clone)]
pub struct MouseGeneric<P, B> {
    pub x_01: P,
    pub y_01: P,
    pub left: B,
    pub right: B,
    pub middle: B,
}

impl<P: Clone, B: Clone> MouseGeneric<P, B> {
    pub fn button(&self, mouse_button: MouseButton) -> B {
        use MouseButton::*;
        match mouse_button {
            Left => self.left.clone(),
            Right => self.right.clone(),
            Middle => self.middle.clone(),
        }
    }

    pub fn x_01(&self) -> P {
        self.x_01.clone()
    }

    pub fn y_01(&self) -> P {
        self.y_01.clone()
    }
}

pub type KeySig = Sig<Variable<bool>>;
pub type Keyboard = KeyboardGeneric<KeySig>;
pub type Mouse = MouseGeneric<Sig<Variable<f32>>, Sig<Variable<bool>>>;

#[derive(Clone)]
pub struct Input {
    pub keyboard: Keyboard,
    pub mouse: Mouse,
}

impl Input {
    pub(crate) fn new() -> Self {
        let mk_key = |_| variable(false);
        let mk_position = || variable(0.0);
        let mk_button = || variable(false);
        Self {
            keyboard: KeyboardGeneric::new(mk_key),
            mouse: MouseGeneric {
                x_01: mk_position(),
                y_01: mk_position(),
                left: mk_button(),
                right: mk_button(),
                middle: mk_button(),
            },
        }
    }

    pub(crate) fn set_key(&self, scancode: Scancode, pressed: bool) {
        if let Some(key_state) =
            caw_sdl2::sdl2_scancode_get(&self.keyboard, scancode)
        {
            key_state.0.set(pressed);
        }
    }

    pub(crate) fn set_mouse_position(&self, x_01: f32, y_01: f32) {
        self.mouse.x_01.0.set(x_01);
        self.mouse.y_01.0.set(y_01);
    }

    pub(crate) fn set_mouse_button(
        &self,
        mouse_button: SdlMouseButton,
        pressed: bool,
    ) {
        let button_state = match mouse_button {
            SdlMouseButton::Left => &self.mouse.left,
            SdlMouseButton::Right => &self.mouse.right,
            SdlMouseButton::Middle => &self.mouse.middle,
            _ => return,
        };
        button_state.0.set(pressed);
    }

    pub fn keyboard(&self) -> Keyboard {
        self.keyboard.clone()
    }

    pub fn mouse(&self) -> Mouse {
        self.mouse.clone()
    }

    pub fn key(&self, key: Key) -> Sig<impl SigT<Item = bool> + use<>> {
        self.keyboard.get(key)
    }

    pub fn mouse_button(
        &self,
        mouse_button: MouseButton,
    ) -> Sig<impl SigT<Item = bool>> {
        self.mouse.button(mouse_button)
    }

    pub fn x_01(&self) -> Sig<impl SigT<Item = f32> + use<>> {
        self.mouse.x_01()
    }

    pub fn y_01(&self) -> Sig<impl SigT<Item = f32> + use<>> {
        self.mouse.y_01()
    }
}
