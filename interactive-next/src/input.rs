pub use caw_computer_keyboard::{Key, Keyboard as KeyboardGeneric};
use caw_core_next::{FrameSig, FrameSigT, SigCtx};
use sdl2::{keyboard::Scancode, mouse::MouseButton as SdlMouseButton};
use std::sync::{Arc, RwLock};

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

impl<P, B> MouseGeneric<P, B> {
    fn map<P_, B_, FP: FnMut(&P) -> P_, FB: FnMut(&B) -> B_>(
        &self,
        mut f_position: FP,
        mut f_button: FB,
    ) -> MouseGeneric<P_, B_> {
        MouseGeneric {
            x_01: f_position(&self.x_01),
            y_01: f_position(&self.y_01),
            left: f_button(&self.left),
            right: f_button(&self.right),
            middle: f_button(&self.middle),
        }
    }
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

/// A signal for keyboard and mouse inputs from sdl. Always yields the same value during any given
/// frame.
pub struct FrameSigInput<T: Copy>(Arc<RwLock<T>>);

impl<T: Copy> FrameSigT for FrameSigInput<T> {
    type Item = T;

    fn frame_sample(&mut self, _ctx: &SigCtx) -> Self::Item {
        *self.0.read().unwrap()
    }
}

impl<T: Copy> Clone for FrameSigInput<T> {
    fn clone(&self) -> Self {
        FrameSigInput(Arc::clone(&self.0))
    }
}

pub type Keyboard = KeyboardGeneric<FrameSig<FrameSigInput<bool>>>;
pub type Mouse =
    MouseGeneric<FrameSig<FrameSigInput<f32>>, FrameSig<FrameSigInput<bool>>>;

#[derive(Clone)]
pub struct Input {
    pub keyboard: Keyboard,
    pub mouse: Mouse,
}

#[derive(Clone)]
pub struct InputState {
    keyboard: KeyboardGeneric<Arc<RwLock<bool>>>,
    mouse: MouseGeneric<Arc<RwLock<f32>>, Arc<RwLock<bool>>>,
}

impl InputState {
    pub(crate) fn new() -> Self {
        let mk_key = || Arc::new(RwLock::new(false));
        let mk_position = || Arc::new(RwLock::new(0.0));
        let mk_button = || Arc::new(RwLock::new(false));
        Self {
            keyboard: KeyboardGeneric {
                a: mk_key(),
                b: mk_key(),
                c: mk_key(),
                d: mk_key(),
                e: mk_key(),
                f: mk_key(),
                g: mk_key(),
                h: mk_key(),
                i: mk_key(),
                j: mk_key(),
                k: mk_key(),
                l: mk_key(),
                m: mk_key(),
                n: mk_key(),
                o: mk_key(),
                p: mk_key(),
                q: mk_key(),
                r: mk_key(),
                s: mk_key(),
                t: mk_key(),
                u: mk_key(),
                v: mk_key(),
                w: mk_key(),
                x: mk_key(),
                y: mk_key(),
                z: mk_key(),
                n0: mk_key(),
                n1: mk_key(),
                n2: mk_key(),
                n3: mk_key(),
                n4: mk_key(),
                n5: mk_key(),
                n6: mk_key(),
                n7: mk_key(),
                n8: mk_key(),
                n9: mk_key(),
                left_bracket: mk_key(),
                right_bracket: mk_key(),
                semicolon: mk_key(),
                apostrophe: mk_key(),
                comma: mk_key(),
                period: mk_key(),
                minus: mk_key(),
                equals: mk_key(),
                slash: mk_key(),
                space: mk_key(),
            },
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
            _ => return,
        };
        *key_state.write().unwrap() = pressed;
    }

    pub(crate) fn set_mouse_position(&self, x_01: f32, y_01: f32) {
        *self.mouse.x_01.write().unwrap() = x_01;
        *self.mouse.y_01.write().unwrap() = y_01;
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
        *button_state.write().unwrap() = pressed;
    }

    pub fn keyboard(&self) -> Keyboard {
        self.keyboard
            .map(|key| FrameSig(FrameSigInput(Arc::clone(key))))
    }

    pub fn mouse(&self) -> Mouse {
        self.mouse.map(
            |position| FrameSig(FrameSigInput(Arc::clone(position))),
            |button| FrameSig(FrameSigInput(Arc::clone(button))),
        )
    }

    pub fn input(&self) -> Input {
        Input {
            keyboard: self.keyboard(),
            mouse: self.mouse(),
        }
    }
}
