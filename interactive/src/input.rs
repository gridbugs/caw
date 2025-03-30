pub use caw_computer_keyboard::{Key, Keyboard as KeyboardGeneric};
use caw_core::{
    frame_sig_var, sig_var, FrameSig, FrameSigT, FrameSigVar, Sig, SigT, SigVar,
};
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

pub type KeyFrameSig = FrameSig<FrameSigVar<bool>>;
pub type KeySig = Sig<SigVar<bool>>;
pub type Keyboard_ = KeyboardGeneric<KeySig>;
pub type Keyboard = KeyboardGeneric<KeyFrameSig>;
pub type Mouse =
    MouseGeneric<FrameSig<FrameSigVar<f32>>, FrameSig<FrameSigVar<bool>>>;
pub type Mouse_ = MouseGeneric<Sig<SigVar<f32>>, Sig<SigVar<bool>>>;

#[derive(Clone)]
pub struct Input_ {
    pub keyboard: Keyboard_,
    pub mouse: Mouse_,
}

#[derive(Clone)]
pub struct Input {
    pub keyboard: Keyboard,
    pub mouse: Mouse,
}

impl Input {
    pub fn key(&self, key: Key) -> FrameSig<impl FrameSigT<Item = bool>> {
        self.keyboard.get(key)
    }

    pub fn mouse_button(
        &self,
        mouse_button: MouseButton,
    ) -> FrameSig<impl FrameSigT<Item = bool>> {
        self.mouse.button(mouse_button)
    }

    pub fn x_01(&self) -> FrameSig<impl FrameSigT<Item = f32>> {
        self.mouse.x_01()
    }

    pub fn y_01(&self) -> FrameSig<impl FrameSigT<Item = f32>> {
        self.mouse.y_01()
    }
}

impl Input_ {
    pub(crate) fn new() -> Self {
        let mk_key = |_| sig_var(false);
        let mk_position = || sig_var(0.0);
        let mk_button = || sig_var(false);
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
        let key_state = match scancode {
            Scancode::A => &self.keyboard.a,
            Scancode::B => &self.keyboard.b,
            Scancode::C => &self.keyboard.c,
            Scancode::D => &self.keyboard.d,
            Scancode::E => &self.keyboard.e,
            Scancode::F => &self.keyboard.f,
            Scancode::G => &self.keyboard.g,
            Scancode::H => &self.keyboard.h,
            Scancode::I => &self.keyboard.i,
            Scancode::J => &self.keyboard.j,
            Scancode::K => &self.keyboard.k,
            Scancode::L => &self.keyboard.l,
            Scancode::M => &self.keyboard.m,
            Scancode::N => &self.keyboard.n,
            Scancode::O => &self.keyboard.o,
            Scancode::P => &self.keyboard.p,
            Scancode::Q => &self.keyboard.q,
            Scancode::R => &self.keyboard.r,
            Scancode::S => &self.keyboard.s,
            Scancode::T => &self.keyboard.t,
            Scancode::U => &self.keyboard.u,
            Scancode::V => &self.keyboard.v,
            Scancode::W => &self.keyboard.w,
            Scancode::X => &self.keyboard.x,
            Scancode::Y => &self.keyboard.y,
            Scancode::Z => &self.keyboard.z,
            Scancode::Num0 => &self.keyboard.n0,
            Scancode::Num1 => &self.keyboard.n1,
            Scancode::Num2 => &self.keyboard.n2,
            Scancode::Num3 => &self.keyboard.n3,
            Scancode::Num4 => &self.keyboard.n4,
            Scancode::Num5 => &self.keyboard.n5,
            Scancode::Num6 => &self.keyboard.n6,
            Scancode::Num7 => &self.keyboard.n7,
            Scancode::Num8 => &self.keyboard.n8,
            Scancode::Num9 => &self.keyboard.n9,
            Scancode::LeftBracket => &self.keyboard.left_bracket,
            Scancode::RightBracket => &self.keyboard.right_bracket,
            Scancode::Semicolon => &self.keyboard.semicolon,
            Scancode::Apostrophe => &self.keyboard.apostrophe,
            Scancode::Comma => &self.keyboard.comma,
            Scancode::Period => &self.keyboard.period,
            Scancode::Minus => &self.keyboard.minus,
            Scancode::Equals => &self.keyboard.equals,
            Scancode::Slash => &self.keyboard.slash,
            Scancode::Space => &self.keyboard.space,
            Scancode::Backspace => &self.keyboard.backspace,
            Scancode::Backslash => &self.keyboard.backslash,
            _ => return,
        };
        key_state.0.set(pressed);
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

    pub fn keyboard(&self) -> Keyboard_ {
        self.keyboard.clone()
    }

    pub fn mouse(&self) -> Mouse_ {
        self.mouse.clone()
    }
}

#[derive(Clone)]
pub struct InputState {
    keyboard: KeyboardGeneric<FrameSig<FrameSigVar<bool>>>,
    mouse:
        MouseGeneric<FrameSig<FrameSigVar<f32>>, FrameSig<FrameSigVar<bool>>>,
}

impl InputState {
    pub(crate) fn new() -> Self {
        let mk_key = |_| frame_sig_var(false);
        let mk_position = || frame_sig_var(0.0);
        let mk_button = || frame_sig_var(false);
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
        let key_state = match scancode {
            Scancode::A => &self.keyboard.a,
            Scancode::B => &self.keyboard.b,
            Scancode::C => &self.keyboard.c,
            Scancode::D => &self.keyboard.d,
            Scancode::E => &self.keyboard.e,
            Scancode::F => &self.keyboard.f,
            Scancode::G => &self.keyboard.g,
            Scancode::H => &self.keyboard.h,
            Scancode::I => &self.keyboard.i,
            Scancode::J => &self.keyboard.j,
            Scancode::K => &self.keyboard.k,
            Scancode::L => &self.keyboard.l,
            Scancode::M => &self.keyboard.m,
            Scancode::N => &self.keyboard.n,
            Scancode::O => &self.keyboard.o,
            Scancode::P => &self.keyboard.p,
            Scancode::Q => &self.keyboard.q,
            Scancode::R => &self.keyboard.r,
            Scancode::S => &self.keyboard.s,
            Scancode::T => &self.keyboard.t,
            Scancode::U => &self.keyboard.u,
            Scancode::V => &self.keyboard.v,
            Scancode::W => &self.keyboard.w,
            Scancode::X => &self.keyboard.x,
            Scancode::Y => &self.keyboard.y,
            Scancode::Z => &self.keyboard.z,
            Scancode::Num0 => &self.keyboard.n0,
            Scancode::Num1 => &self.keyboard.n1,
            Scancode::Num2 => &self.keyboard.n2,
            Scancode::Num3 => &self.keyboard.n3,
            Scancode::Num4 => &self.keyboard.n4,
            Scancode::Num5 => &self.keyboard.n5,
            Scancode::Num6 => &self.keyboard.n6,
            Scancode::Num7 => &self.keyboard.n7,
            Scancode::Num8 => &self.keyboard.n8,
            Scancode::Num9 => &self.keyboard.n9,
            Scancode::LeftBracket => &self.keyboard.left_bracket,
            Scancode::RightBracket => &self.keyboard.right_bracket,
            Scancode::Semicolon => &self.keyboard.semicolon,
            Scancode::Apostrophe => &self.keyboard.apostrophe,
            Scancode::Comma => &self.keyboard.comma,
            Scancode::Period => &self.keyboard.period,
            Scancode::Minus => &self.keyboard.minus,
            Scancode::Equals => &self.keyboard.equals,
            Scancode::Slash => &self.keyboard.slash,
            Scancode::Space => &self.keyboard.space,
            Scancode::Backspace => &self.keyboard.backspace,
            Scancode::Backslash => &self.keyboard.backslash,
            _ => return,
        };
        key_state.0.set(pressed);
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

    pub fn input(&self) -> Input {
        Input {
            keyboard: self.keyboard(),
            mouse: self.mouse(),
        }
    }
}
