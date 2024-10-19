use caw_core_next::{Gate, Signal};

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
    Space,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Clone)]
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
    pub space: Key,
}

#[derive(Clone)]
pub struct MouseGeneric<Position, Button> {
    pub x_01: Position,
    pub y_01: Position,
    pub left: Button,
    pub right: Button,
    pub middle: Button,
}

impl<G: Gate + Clone> KeyboardGeneric<G> {
    pub fn get(&self, key: Key) -> impl Gate {
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

impl<P: Signal<Item = f64> + Clone, G: Gate + Clone> MouseGeneric<P, G> {
    pub fn button(&self, mouse_button: MouseButton) -> impl Gate {
        use MouseButton::*;
        match mouse_button {
            Left => self.left.clone(),
            Right => self.right.clone(),
            Middle => self.middle.clone(),
        }
    }

    pub fn x_01(&self) -> impl Signal<Item = f64> {
        self.x_01.clone()
    }

    pub fn y_01(&self) -> impl Signal<Item = f64> {
        self.y_01.clone()
    }
}
