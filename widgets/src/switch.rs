use crate::window::{TitlePosition, Window};
use anyhow::anyhow;
use caw_persist::PersistData;
use coord_2d::Coord;
use sdl2::{event::WindowEvent, keyboard::Scancode, pixels::Color, rect::Rect};
use serde::{Deserialize, Serialize};
use std::time::Instant;

const WIDTH_PX: u32 = 128;
const HEIGHT_PX: u32 = 128;

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
struct State {
    pressed: bool,
}

impl PersistData for State {
    const NAME: &'static str = "switch_state";
}

struct UiButton {
    rect: Rect,
    text: String,
}

impl UiButton {
    fn contains(&self, mouse_position: Coord) -> bool {
        self.rect.contains_point(mouse_position)
    }

    fn render(
        &self,
        mouse_position: Coord,
        background: Option<Color>,
        window: &mut Window,
    ) -> anyhow::Result<()> {
        let background = match background {
            Some(background) => background,
            None => {
                let x = if self.rect.contains_point(mouse_position) {
                    128
                } else {
                    64
                };
                (x, x, x).into()
            }
        };
        window.canvas.set_draw_color(background);
        window
            .canvas
            .fill_rect(self.rect)
            .map_err(|e| anyhow!("{e}"))?;
        window.canvas.set_draw_color(Color::WHITE);
        window
            .canvas
            .draw_rect(self.rect)
            .map_err(|e| anyhow!("{e}"))?;
        let text_surface = window
            .font
            .render(self.text.as_str())
            .blended(Color::WHITE)
            .map_err(|e| anyhow!("{e}"))?;
        let text_texture = text_surface.as_texture(&window.texture_creator)?;
        let text_texture_query = text_texture.query();
        window
            .canvas
            .copy(
                &text_texture,
                None,
                Some(Rect::new(
                    (window.width_px() as i32 / 2)
                        - (text_texture_query.width as i32 / 2),
                    self.rect.y,
                    text_texture_query.width,
                    text_texture_query.height,
                )),
            )
            .map_err(|e| anyhow!("{e}"))?;
        Ok(())
    }
}

struct UiButtons {
    on: UiButton,
    off: UiButton,
    toggle: UiButton,
}

pub struct Switch {
    window: Window,
    state: State,
    ui_buttons: UiButtons,
    mouse_position: Coord,
}

impl Switch {
    pub fn new(title: Option<&str>) -> anyhow::Result<Self> {
        let window = Window::new(title, WIDTH_PX, HEIGHT_PX)?;
        let state = if let Some(state) = title.and_then(|t| State::load_(t)) {
            state
        } else {
            State { pressed: false }
        };
        let height = 20;
        let width = 80;
        let padding_y = 8;
        let padding_x = ((WIDTH_PX - width) / 2) as i32;
        Ok(Self {
            window,
            state,
            ui_buttons: UiButtons {
                on: UiButton {
                    rect: Rect::new(padding_x, padding_y, width, height),
                    text: "On".to_string(),
                },
                off: UiButton {
                    rect: Rect::new(
                        padding_x,
                        padding_y * 2 + height as i32,
                        width,
                        height,
                    ),
                    text: "Off".to_string(),
                },
                toggle: UiButton {
                    rect: Rect::new(
                        padding_x,
                        padding_y * 3 + height as i32 * 2,
                        width,
                        height,
                    ),
                    text: "Toggle".to_string(),
                },
            },
            mouse_position: Coord::new(0, 0),
        })
    }

    fn handle_events(&mut self) {
        let prev_state = self.state;
        for event in self.window.event_pump.poll_iter() {
            use sdl2::event::Event;
            Window::handle_event_common(
                event.clone(),
                self.window.title.as_ref(),
            );
            match event {
                Event::MouseMotion { x, y, .. } => {
                    self.mouse_position = Coord::new(x, y);
                }
                Event::KeyDown {
                    scancode: Some(Scancode::Space),
                    ..
                } => self.state.pressed = !self.state.pressed,
                Event::MouseButtonDown { x, y, .. } => {
                    let coord = Coord::new(x, y);
                    if self.ui_buttons.on.contains(coord) {
                        self.state.pressed = true;
                    } else if self.ui_buttons.off.contains(coord) {
                        self.state.pressed = false;
                    } else if self.ui_buttons.toggle.contains(coord) {
                        self.state.pressed = !self.state.pressed;
                    }
                }
                Event::Window {
                    win_event: WindowEvent::FocusGained,
                    ..
                } => {
                    let coord = self.mouse_position;
                    if self.ui_buttons.on.contains(coord) {
                        self.state.pressed = true;
                    } else if self.ui_buttons.off.contains(coord) {
                        self.state.pressed = false;
                    } else if self.ui_buttons.toggle.contains(coord) {
                        self.state.pressed = !self.state.pressed;
                    }
                }
                _ => (),
            }
        }
        if let Some(title) = self.window.title.as_ref() {
            if prev_state != self.state {
                self.state.save_(title);
            }
        }
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.window.canvas.set_draw_color(Color::BLACK);
        self.window.canvas.clear();
        self.window.render_title(TitlePosition::CenterBottom)?;
        let (on_background, off_background) = if self.state.pressed {
            (Some((0, 128, 0).into()), None)
        } else {
            (None, Some((128, 0, 0).into()))
        };
        self.ui_buttons.on.render(
            self.mouse_position,
            on_background,
            &mut self.window,
        )?;
        self.ui_buttons.off.render(
            self.mouse_position,
            off_background,
            &mut self.window,
        )?;
        self.ui_buttons.toggle.render(
            self.mouse_position,
            None,
            &mut self.window,
        )?;
        self.window.canvas.present();
        Ok(())
    }

    fn update(&mut self) -> anyhow::Result<()> {
        self.handle_events();
        self.render()?;
        Ok(())
    }

    /// Waits until the next frame, then handles events and redraws the widget
    pub fn tick(&mut self) -> anyhow::Result<()> {
        self.window.wait_until_next_frame();
        self.update()?;
        self.window.prev_tick_complete = Instant::now();
        Ok(())
    }

    pub fn pressed(&self) -> bool {
        self.state.pressed
    }
}
