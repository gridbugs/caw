use caw_core_next::{FrameSig, FrameSigT};
use caw_keyboard::{KeyEvent, KeyEvents, Note};

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
    Backspace,
    Backslash,
}

#[derive(Clone)]
pub struct Keyboard<K> {
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
    pub backspace: K,
    pub backslash: K,
}

impl<K> Keyboard<K>
where
    K: Clone,
{
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
            Backspace => self.backspace.clone(),
            Backslash => self.backslash.clone(),
        }
    }
}

impl<K> Keyboard<K> {
    pub fn map<K_, F: FnMut(&K) -> K_>(&self, mut f: F) -> Keyboard<K_> {
        Keyboard {
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
            backspace: f(&self.backspace),
            backslash: f(&self.backslash),
        }
    }
}

/// Maps the keys on a US keyboard to musical notes. The 4 rows from the number row to the ZXCV row
/// are used, where the QWER and ZXCV are white notes and the number and ZXCV rows are black notes.
fn opinionated_note_by_key(start_note: Note) -> Vec<(Key, Note)> {
    use Key::*;
    let top_row_base = start_note.add_octaves(1);
    let top_row = vec![
        Q,
        W,
        N3,
        E,
        N4,
        R,
        T,
        N6,
        Y,
        N7,
        U,
        N8,
        I,
        O,
        N0,
        P,
        Minus,
        LeftBracket,
        RightBracket,
        Backspace,
        Backslash,
    ];
    let bottom_row = vec![Z, X, D, C, F, V, B, H, N, J, M, K, Comma, Period];
    top_row
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, top_row_base.add_semitones(i as i16)))
        .chain(
            bottom_row
                .into_iter()
                .enumerate()
                .map(|(i, key)| (key, start_note.add_semitones(i as i16))),
        )
        .collect::<Vec<_>>()
}

pub fn opinionated_key_events<S>(
    keyboard: &Keyboard<S>,
    start_note: Note,
) -> FrameSig<impl FrameSigT<Item = KeyEvents>>
where
    S: FrameSigT<Item = bool> + Clone,
{
    struct KeyState<S>
    where
        S: FrameSigT<Item = bool> + Clone,
    {
        key_sig: S,
        note: Note,
        pressed: bool,
    }
    let note_by_key = opinionated_note_by_key(start_note);
    let mut state = note_by_key
        .into_iter()
        .map(|(key, note)| KeyState {
            key_sig: keyboard.get(key),
            note,
            pressed: false,
        })
        .collect::<Vec<_>>();
    FrameSig::from_fn(move |ctx| {
        let mut key_events = KeyEvents::empty();
        for state in state.iter_mut() {
            let pressed = state.key_sig.frame_sample(ctx);
            if pressed != state.pressed {
                state.pressed = pressed;
                key_events.push(KeyEvent {
                    note: state.note,
                    pressed,
                    velocity_01: 1.0,
                });
            }
        }
        key_events
    })
}

impl<K> Keyboard<K>
where
    K: FrameSigT<Item = bool> + Clone,
{
    pub fn opinionated_key_events(
        self,
        start_note: Note,
    ) -> FrameSig<impl FrameSigT<Item = KeyEvents>> {
        opinionated_key_events(&self, start_note)
    }
}
