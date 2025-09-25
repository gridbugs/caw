use crate::window::{TitlePosition, Window};
use anyhow::anyhow;
use caw_persist::PersistData;
use line_2d::Coord;
use midly::num::u7;
use sdl2::{
    event::Event, keyboard::Scancode, mouse::MouseButton, pixels::Color,
    rect::Rect,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

const WIDTH_PX: u32 = 128;
const HEIGHT_PX: u32 = 128;

const RANGE_RADS: f32 = (std::f32::consts::PI * 3.0) / 2.0;

#[derive(Serialize, Deserialize)]
struct State {
    value_01: f32,
}

impl PersistData for State {
    const NAME: &'static str = "knob_state";
}

pub struct Knob {
    title: Option<String>,
    window: Window,
    state: State,
    space_pressed: bool,
    sensitivity: f32,
}

impl Knob {
    pub fn new(
        title: Option<&str>,
        initial_value_01: f32,
        sensitivity: f32,
    ) -> anyhow::Result<Self> {
        let window = Window::new(title, WIDTH_PX, HEIGHT_PX)?;
        let state = if let Some(state) = title.and_then(|t| State::load_(t)) {
            state
        } else {
            State {
                value_01: initial_value_01.clamp(0., 1.),
            }
        };
        Ok(Self {
            title: title.map(|s| s.to_string()),
            window,
            state,
            space_pressed: false,
            sensitivity,
        })
    }

    fn handle_events(&mut self) {
        for event in self.window.event_pump.poll_iter() {
            Window::handle_event_common(
                event.clone(),
                self.window.title.as_ref(),
            );
            match event {
                Event::KeyDown { scancode, .. } => {
                    let value_01 = match scancode {
                        Some(Scancode::Grave) => 0.0,
                        Some(Scancode::Num1) => 0.1,
                        Some(Scancode::Num2) => 0.2,
                        Some(Scancode::Num3) => 0.3,
                        Some(Scancode::Num4) => 0.4,
                        Some(Scancode::Num5) => 0.5,
                        Some(Scancode::Num6) => 0.6,
                        Some(Scancode::Num7) => 0.7,
                        Some(Scancode::Num8) => 0.8,
                        Some(Scancode::Num9) => 0.9,
                        Some(Scancode::Num0) => 1.0,
                        Some(Scancode::Space) => {
                            self.space_pressed = true;
                            continue;
                        }
                        _ => continue,
                    };
                    self.state.value_01 = value_01;
                }
                Event::KeyUp { scancode, .. } => match scancode {
                    Some(Scancode::Space) => self.space_pressed = false,
                    _ => (),
                },
                Event::MouseWheel { precise_y, .. } => {
                    let multiplier = 0.1;
                    self.state.value_01 = (self.state.value_01
                        + (precise_y * multiplier * self.sensitivity))
                        .clamp(0., 1.);
                }
                Event::MouseMotion {
                    mousestate, yrel, ..
                } => {
                    if mousestate.is_mouse_button_pressed(MouseButton::Left) {
                        let multiplier = -0.05;
                        self.state.value_01 = (self.state.value_01
                            + (yrel as f32 * multiplier * self.sensitivity))
                            .clamp(0., 1.);
                    }
                }
                _ => (),
            }
        }
        if let Some(title) = self.title.as_ref() {
            self.state.save_(title);
        }
    }

    fn render_value(&mut self) -> anyhow::Result<()> {
        let text_surface = self
            .window
            .font
            .render(format!("{}", self.state.value_01).as_str())
            .blended(Color::WHITE)
            .map_err(|e| anyhow!("{e}"))?;
        let text_texture =
            text_surface.as_texture(&self.window.texture_creator)?;
        let (canvas_width, canvas_height) = self
            .window
            .canvas
            .output_size()
            .map_err(|e| anyhow!("{e}"))?;
        let text_texture_query = text_texture.query();
        // Render the title centred at the bottom of the window.
        let text_rect = Rect::new(
            (canvas_width as i32 - text_texture_query.width as i32) / 2,
            canvas_height as i32 - text_texture_query.height as i32,
            text_texture_query.width,
            text_texture_query.height,
        );
        self.window
            .canvas
            .copy(&text_texture, None, Some(text_rect))
            .map_err(|e| anyhow!("{e}"))?;
        Ok(())
    }

    fn render_knob(&mut self) -> anyhow::Result<()> {
        let (available_width, canvas_height) = self
            .window
            .canvas
            .output_size()
            .map_err(|e| anyhow!("{e}"))?;
        let text_padding_px = 30;
        let available_height = canvas_height - text_padding_px;
        let centre = Coord {
            x: available_width as i32 / 2,
            y: available_height as i32 / 2,
        };
        let absolute_rotation_rads =
            (((std::f32::consts::PI * 2.0) - RANGE_RADS) / 2.0)
                + std::f32::consts::FRAC_PI_2;
        let relative_angle_rads = RANGE_RADS * self.state.value_01;
        let line_length_px =
            ((available_width.min(available_height) / 2) - 10) as f32;
        let angle_rads = absolute_rotation_rads + relative_angle_rads;
        let end_dx = angle_rads.cos() * line_length_px;
        let end_dy = angle_rads.sin() * line_length_px;
        let end = centre
            + Coord {
                x: end_dx as i32,
                y: end_dy as i32,
            };
        let line_width = 4;
        self.window.canvas.set_draw_color(Color::WHITE);
        for Coord { x, y } in line_2d::coords_between(centre, end) {
            let rect = Rect::new(
                x - (line_width as i32 / 2),
                y - (line_width as i32 / 2),
                line_width,
                line_width,
            );
            self.window
                .canvas
                .fill_rect(rect)
                .map_err(|e| anyhow!("{e}"))?;
        }
        Ok(())
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.window.canvas.set_draw_color(Color::BLACK);
        self.window.canvas.clear();
        self.window.render_title(TitlePosition::CenterBottom)?;
        self.render_value()?;
        self.render_knob()?;
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

    pub fn value_01(&self) -> f32 {
        self.state.value_01
    }

    pub fn value_midi(&self) -> u7 {
        ((self.value_01() * u7::max_value().as_int() as f32) as u8).into()
    }

    pub fn is_space_pressed(&self) -> bool {
        self.space_pressed
    }
}
