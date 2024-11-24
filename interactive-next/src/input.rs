pub use caw_computer_keyboard::Key;
use caw_core_next::{Buf, ConstBuf, Sig, SigCtx, SigT};
use sdl2::{keyboard::Scancode, mouse::MouseButton as SdlMouseButton};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Clone)]
pub struct KeyboardGeneric<K> {
    pub a: K,
    pub b: K,
    pub c: K,
    pub d: K,
    pub e: K,
    pub f: K,
    pub g: K,
    pub h: K,
    pub i: K,
    pub j: K,
    pub k: K,
    pub l: K,
    pub m: K,
    pub n: K,
    pub o: K,
    pub p: K,
    pub q: K,
    pub r: K,
    pub s: K,
    pub t: K,
    pub u: K,
    pub v: K,
    pub w: K,
    pub x: K,
    pub y: K,
    pub z: K,
    pub n0: K,
    pub n1: K,
    pub n2: K,
    pub n3: K,
    pub n4: K,
    pub n5: K,
    pub n6: K,
    pub n7: K,
    pub n8: K,
    pub n9: K,
    pub left_bracket: K,
    pub right_bracket: K,
    pub semicolon: K,
    pub apostrophe: K,
    pub comma: K,
    pub period: K,
    pub minus: K,
    pub equals: K,
    pub slash: K,
    pub space: K,
}

#[derive(Clone)]
pub struct MouseGeneric<P, B> {
    pub x_01: P,
    pub y_01: P,
    pub left: B,
    pub right: B,
    pub middle: B,
}

impl<K: Clone> KeyboardGeneric<K> {
    pub fn get(&self, key: Key) -> K {
        use Key::*;
        match key {
            A => self.a.clone(),
            B => self.b.clone(),
            C => self.c.clone(),
            D => self.d.clone(),
            E => self.e.clone(),
            F => self.f.clone(),
            G => self.g.clone(),
            H => self.h.clone(),
            I => self.i.clone(),
            J => self.j.clone(),
            K => self.k.clone(),
            L => self.l.clone(),
            M => self.m.clone(),
            N => self.n.clone(),
            O => self.o.clone(),
            P => self.p.clone(),
            Q => self.q.clone(),
            R => self.r.clone(),
            S => self.s.clone(),
            T => self.t.clone(),
            U => self.u.clone(),
            V => self.v.clone(),
            W => self.w.clone(),
            X => self.x.clone(),
            Y => self.y.clone(),
            Z => self.z.clone(),
            N0 => self.n0.clone(),
            N1 => self.n1.clone(),
            N2 => self.n2.clone(),
            N3 => self.n3.clone(),
            N4 => self.n4.clone(),
            N5 => self.n5.clone(),
            N6 => self.n6.clone(),
            N7 => self.n7.clone(),
            N8 => self.n8.clone(),
            N9 => self.n9.clone(),
            LeftBracket => self.left_bracket.clone(),
            RightBracket => self.right_bracket.clone(),
            Semicolon => self.semicolon.clone(),
            Apostrophe => self.apostrophe.clone(),
            Comma => self.comma.clone(),
            Period => self.period.clone(),
            Minus => self.minus.clone(),
            Equals => self.equals.clone(),
            Slash => self.slash.clone(),
            Space => self.space.clone(),
        }
    }
}

impl<K> KeyboardGeneric<K> {
    fn map<K_, F: FnMut(&K) -> K_>(&self, mut f: F) -> KeyboardGeneric<K_> {
        KeyboardGeneric {
            a: f(&self.a),
            b: f(&self.b),
            c: f(&self.c),
            d: f(&self.d),
            e: f(&self.e),
            f: f(&self.f),
            g: f(&self.g),
            h: f(&self.h),
            i: f(&self.i),
            j: f(&self.j),
            k: f(&self.k),
            l: f(&self.l),
            m: f(&self.m),
            n: f(&self.n),
            o: f(&self.o),
            p: f(&self.p),
            q: f(&self.q),
            r: f(&self.r),
            s: f(&self.s),
            t: f(&self.t),
            u: f(&self.u),
            v: f(&self.v),
            w: f(&self.w),
            x: f(&self.x),
            y: f(&self.y),
            z: f(&self.z),
            n0: f(&self.n0),
            n1: f(&self.n1),
            n2: f(&self.n2),
            n3: f(&self.n3),
            n4: f(&self.n4),
            n5: f(&self.n5),
            n6: f(&self.n6),
            n7: f(&self.n7),
            n8: f(&self.n8),
            n9: f(&self.n9),
            left_bracket: f(&self.left_bracket),
            right_bracket: f(&self.right_bracket),
            semicolon: f(&self.semicolon),
            apostrophe: f(&self.apostrophe),
            comma: f(&self.comma),
            period: f(&self.period),
            minus: f(&self.minus),
            equals: f(&self.equals),
            slash: f(&self.slash),
            space: f(&self.space),
        }
    }
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
pub struct SigInput<T: Copy>(Arc<RwLock<T>>);
impl<T: Copy> SigT for SigInput<T> {
    type Item = T;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: *self.0.read().unwrap(),
            count: ctx.num_samples,
        }
    }
}

impl<T: Copy> Clone for SigInput<T> {
    fn clone(&self) -> Self {
        SigInput(Arc::clone(&self.0))
    }
}

pub type Keyboard = KeyboardGeneric<Sig<SigInput<bool>>>;
pub type Mouse = MouseGeneric<Sig<SigInput<f32>>, Sig<SigInput<bool>>>;

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
        self.keyboard.map(|key| Sig(SigInput(Arc::clone(key))))
    }

    pub fn mouse(&self) -> Mouse {
        self.mouse.map(
            |position| Sig(SigInput(Arc::clone(position))),
            |button| Sig(SigInput(Arc::clone(button))),
        )
    }

    pub fn input(&self) -> Input {
        Input {
            keyboard: self.keyboard(),
            mouse: self.mouse(),
        }
    }
}
