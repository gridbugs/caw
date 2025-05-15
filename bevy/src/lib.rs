use bevy::{input::keyboard::KeyboardInput, prelude::*, window::PrimaryWindow};
use caw_computer_keyboard::Keyboard;
use caw_core::{Sig, SigVar, sig_var};

#[derive(Resource, Clone)]
pub struct BevyInput {
    pub mouse_x: Sig<SigVar<f32>>,
    pub mouse_y: Sig<SigVar<f32>>,
    pub keyboard: Keyboard<Sig<SigVar<bool>>>,
}

impl Default for BevyInput {
    fn default() -> Self {
        let keyboard = Keyboard::new(|_| sig_var(false));
        let mouse_x = sig_var(0.5);
        let mouse_y = sig_var(0.5);
        Self {
            keyboard,
            mouse_x,
            mouse_y,
        }
    }
}

impl BevyInput {
    pub fn update(
        input: Res<BevyInput>,
        evr_kbd: EventReader<KeyboardInput>,
        window: Query<&Window, With<PrimaryWindow>>,
    ) {
        update_input(input, evr_kbd, window);
    }
}

fn get_key<K>(key_code: KeyCode, keyboard: &Keyboard<K>) -> Option<&K> {
    Some(match key_code {
        KeyCode::KeyA => &keyboard.a,
        KeyCode::KeyB => &keyboard.b,
        KeyCode::KeyC => &keyboard.c,
        KeyCode::KeyD => &keyboard.d,
        KeyCode::KeyE => &keyboard.e,
        KeyCode::KeyF => &keyboard.f,
        KeyCode::KeyG => &keyboard.g,
        KeyCode::KeyH => &keyboard.h,
        KeyCode::KeyI => &keyboard.i,
        KeyCode::KeyJ => &keyboard.j,
        KeyCode::KeyK => &keyboard.k,
        KeyCode::KeyL => &keyboard.l,
        KeyCode::KeyM => &keyboard.m,
        KeyCode::KeyN => &keyboard.n,
        KeyCode::KeyO => &keyboard.o,
        KeyCode::KeyP => &keyboard.p,
        KeyCode::KeyQ => &keyboard.q,
        KeyCode::KeyR => &keyboard.r,
        KeyCode::KeyS => &keyboard.s,
        KeyCode::KeyT => &keyboard.t,
        KeyCode::KeyU => &keyboard.u,
        KeyCode::KeyV => &keyboard.v,
        KeyCode::KeyW => &keyboard.w,
        KeyCode::KeyX => &keyboard.x,
        KeyCode::KeyY => &keyboard.y,
        KeyCode::KeyZ => &keyboard.z,
        KeyCode::Digit0 => &keyboard.n0,
        KeyCode::Digit1 => &keyboard.n1,
        KeyCode::Digit2 => &keyboard.n2,
        KeyCode::Digit3 => &keyboard.n3,
        KeyCode::Digit4 => &keyboard.n4,
        KeyCode::Digit5 => &keyboard.n5,
        KeyCode::Digit6 => &keyboard.n6,
        KeyCode::Digit7 => &keyboard.n7,
        KeyCode::Digit8 => &keyboard.n8,
        KeyCode::Digit9 => &keyboard.n9,
        KeyCode::BracketLeft => &keyboard.left_bracket,
        KeyCode::BracketRight => &keyboard.right_bracket,
        KeyCode::Semicolon => &keyboard.semicolon,
        KeyCode::Quote => &keyboard.apostrophe,
        KeyCode::Comma => &keyboard.comma,
        KeyCode::Period => &keyboard.period,
        KeyCode::Minus => &keyboard.minus,
        KeyCode::Equal => &keyboard.equals,
        KeyCode::Slash => &keyboard.slash,
        KeyCode::Space => &keyboard.space,
        KeyCode::Backspace => &keyboard.backspace,
        KeyCode::Backslash => &keyboard.backslash,
        _ => return None,
    })
}

fn update_key_state(
    input: &KeyboardInput,
    keyboard: &Keyboard<Sig<SigVar<bool>>>,
) {
    get_key(input.key_code, keyboard)
        .iter()
        .for_each(|key_state| {
            use bevy::input::ButtonState;
            let pressed = match input.state {
                ButtonState::Pressed => true,
                ButtonState::Released => false,
            };
            key_state.0.set(pressed);
        });
}

fn update_mouse_position(
    window: &Window,
    mouse_x: &Sig<SigVar<f32>>,
    mouse_y: &Sig<SigVar<f32>>,
) {
    if let Some(position) = window.cursor_position() {
        let window_size = window.size();
        let position_01 = position / window_size;
        mouse_x.0.set(position_01.x);
        mouse_y.0.set(position_01.y);
    }
}

fn update_input(
    input: Res<BevyInput>,
    mut evr_kbd: EventReader<KeyboardInput>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let window = window.single().expect("There is more than one window!");
    for ev in evr_kbd.read() {
        update_key_state(ev, &input.keyboard);
    }
    update_mouse_position(window, &input.mouse_x, &input.mouse_y);
}
