use crate::signal::{Gate, Sf64, Signal};
use sdl2::{keyboard::Scancode, mouse::MouseButton as SdlMouseButton};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone, Copy)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    N0,
    N1,
    N2,
    N3,
    N4,
    N5,
    N6,
    N7,
    N8,
    N9,
    LeftBracket,
    RightBracket,
    Semicolon,
    Apostrophe,
    Comma,
    Period,
    Minus,
    Equals,
    Slash,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub struct KeyboardGeneric<Key> {
    pub a: Key,
    pub b: Key,
    pub c: Key,
    pub d: Key,
    pub e: Key,
    pub f: Key,
    pub g: Key,
    pub h: Key,
    pub i: Key,
    pub j: Key,
    pub k: Key,
    pub l: Key,
    pub m: Key,
    pub n: Key,
    pub o: Key,
    pub p: Key,
    pub q: Key,
    pub r: Key,
    pub s: Key,
    pub t: Key,
    pub u: Key,
    pub v: Key,
    pub w: Key,
    pub x: Key,
    pub y: Key,
    pub z: Key,
    pub n0: Key,
    pub n1: Key,
    pub n2: Key,
    pub n3: Key,
    pub n4: Key,
    pub n5: Key,
    pub n6: Key,
    pub n7: Key,
    pub n8: Key,
    pub n9: Key,
    pub left_bracket: Key,
    pub right_bracket: Key,
    pub semicolon: Key,
    pub apostrophe: Key,
    pub comma: Key,
    pub period: Key,
    pub minus: Key,
    pub equals: Key,
    pub slash: Key,
}

pub struct MouseGeneric<Position, Button> {
    pub x_01: Position,
    pub y_01: Position,
    pub left: Button,
    pub right: Button,
    pub middle: Button,
}

pub type Keyboard = KeyboardGeneric<Gate>;
pub type Mouse = MouseGeneric<Sf64, Gate>;

impl Keyboard {
    pub fn get(&self, key: Key) -> Gate {
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
        }
    }
}

impl Mouse {
    pub fn button(&self, mouse_button: MouseButton) -> Gate {
        use MouseButton::*;
        match mouse_button {
            Left => self.left.clone(),
            Right => self.right.clone(),
            Middle => self.middle.clone(),
        }
    }

    pub fn x_01(&self) -> Sf64 {
        self.x_01.clone()
    }

    pub fn y_01(&self) -> Sf64 {
        self.y_01.clone()
    }
}

pub struct Input {
    pub keyboard: Keyboard,
    pub mouse: Mouse,
}

impl Input {
    pub fn key(&self, key: Key) -> Gate {
        self.keyboard.get(key)
    }

    pub fn mouse_button(&self, mouse_button: MouseButton) -> Gate {
        self.mouse.button(mouse_button)
    }

    pub fn x_01(&self) -> Sf64 {
        self.mouse.x_01()
    }

    pub fn y_01(&self) -> Sf64 {
        self.mouse.y_01()
    }
}

pub struct InputState {
    keyboard: KeyboardGeneric<Rc<RefCell<bool>>>,
    mouse: MouseGeneric<Rc<RefCell<f64>>, Rc<RefCell<bool>>>,
}

impl InputState {
    pub fn new() -> Self {
        let mk_key = || Rc::new(RefCell::new(false));
        let mk_position = || Rc::new(RefCell::new(0.0));
        let mk_button = || Rc::new(RefCell::new(false));
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

    pub fn set_key(&self, scancode: Scancode, pressed: bool) {
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
            _ => return,
        };
        *key_state.borrow_mut() = pressed;
    }

    pub fn set_mouse_position(&self, x_01: f64, y_01: f64) {
        *self.mouse.x_01.borrow_mut() = x_01;
        *self.mouse.y_01.borrow_mut() = y_01;
    }

    pub fn set_mouse_button(&self, mouse_button: SdlMouseButton, pressed: bool) {
        let button_state = match mouse_button {
            SdlMouseButton::Left => &self.mouse.left,
            SdlMouseButton::Right => &self.mouse.right,
            SdlMouseButton::Middle => &self.mouse.middle,
            _ => return,
        };
        *button_state.borrow_mut() = pressed;
    }

    pub fn keyboard(&self) -> Keyboard {
        self.keyboard.map(|key| {
            let key = Rc::clone(key);
            Gate::from_fn(move |_| *key.borrow())
        })
    }

    pub fn mouse(&self) -> Mouse {
        self.mouse.map(
            |position| {
                let position = Rc::clone(position);
                Signal::from_fn(move |_| *position.borrow())
            },
            |button| {
                let button = Rc::clone(button);
                Gate::from_fn(move |_| *button.borrow())
            },
        )
    }

    pub fn input(&self) -> Input {
        Input {
            keyboard: self.keyboard(),
            mouse: self.mouse(),
        }
    }
}

impl<Key> KeyboardGeneric<Key> {
    fn map<Key_, F: FnMut(&Key) -> Key_>(&self, mut f: F) -> KeyboardGeneric<Key_> {
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
        }
    }
}

impl<Position, Button> MouseGeneric<Position, Button> {
    fn map<
        Position_,
        Button_,
        FPosition: FnMut(&Position) -> Position_,
        FButton: FnMut(&Button) -> Button_,
    >(
        &self,
        mut f_position: FPosition,
        mut f_button: FButton,
    ) -> MouseGeneric<Position_, Button_> {
        MouseGeneric {
            x_01: f_position(&self.x_01),
            y_01: f_position(&self.y_01),
            left: f_button(&self.left),
            right: f_button(&self.right),
            middle: f_button(&self.middle),
        }
    }
}
