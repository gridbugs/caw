use crate::window::{TitlePosition, Window};
use anyhow::anyhow;
use caw_persistent::PersistentData;
use midly::num::u7;
use sdl2::{
    gfx::rotozoom::RotozoomSurface, keyboard::Scancode, mouse::MouseButton,
    pixels::Color, rect::Rect,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

const WIDTH_PX: u32 = 128;
const HEIGHT_PX: u32 = 128;

#[derive(Serialize, Deserialize)]
struct State {
    x_px: u32,
    y_px: u32,
}

impl PersistentData for State {
    const NAME: &'static str = "xy_state";
}

#[derive(Debug)]
pub struct AxisLabels {
    pub x: Option<String>,
    pub y: Option<String>,
}

pub struct Xy {
    window: Window,
    state: State,
    space_pressed: bool,
    axis_labels: AxisLabels,
}

impl Xy {
    pub fn new(
        title: Option<&str>,
        axis_labels: AxisLabels,
    ) -> anyhow::Result<Self> {
        let window = Window::new(title, WIDTH_PX, HEIGHT_PX)?;
        let state = if let Some(state) = title.and_then(|t| State::load_(t)) {
            state
        } else {
            State {
                x_px: WIDTH_PX / 2,
                y_px: HEIGHT_PX / 2,
            }
        };
        Ok(Self {
            window,
            state,
            space_pressed: false,
            axis_labels,
        })
    }

    fn handle_events(&mut self) {
        for event in self.window.event_pump.poll_iter() {
            use sdl2::event::Event;
            Window::handle_event_common(
                event.clone(),
                self.window.title.as_ref(),
            );
            match event {
                Event::KeyDown { scancode, .. } => match scancode {
                    Some(Scancode::Space) => self.space_pressed = true,
                    _ => (),
                },
                Event::KeyUp { scancode, .. } => match scancode {
                    Some(Scancode::Space) => self.space_pressed = false,
                    _ => (),
                },
                Event::MouseButtonDown { x, y, .. } => {
                    self.state.x_px = x.clamp(0, WIDTH_PX as i32) as u32;
                    self.state.y_px = y.clamp(0, HEIGHT_PX as i32) as u32;
                }
                Event::MouseMotion {
                    mousestate, x, y, ..
                } => {
                    if mousestate.is_mouse_button_pressed(MouseButton::Left) {
                        self.state.x_px = x.clamp(0, WIDTH_PX as i32) as u32;
                        self.state.y_px = y.clamp(0, HEIGHT_PX as i32) as u32;
                    }
                }
                _ => (),
            }
        }
        if let Some(title) = self.window.title.as_ref() {
            self.state.save_(title);
        }
    }

    fn render_xy(&mut self) -> anyhow::Result<()> {
        self.window.canvas.set_draw_color(Color::GREY);
        let line_width = 2;
        let rect_horizontal = Rect::new(
            0,
            self.state.y_px as i32 - line_width as i32 / 2,
            self.window.width_px(),
            line_width,
        );
        let rect_vertical = Rect::new(
            self.state.x_px as i32 - line_width as i32 / 2,
            0,
            line_width,
            self.window.height_px(),
        );
        self.window
            .canvas
            .fill_rect(rect_horizontal)
            .map_err(|e| anyhow!("{e}"))?;
        self.window
            .canvas
            .fill_rect(rect_vertical)
            .map_err(|e| anyhow!("{e}"))?;
        Ok(())
    }

    fn render_axes(
        &mut self,
        x: Option<&str>,
        y: Option<&str>,
        ortho_padding_px: i32,
    ) -> anyhow::Result<()> {
        let padding_px = 10;
        if let Some(x) = x {
            let text_surface = self
                .window
                .font
                .render(x)
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
            let x_position = canvas_width as i32
                - text_texture_query.width as i32
                - padding_px;
            let y_position = canvas_height as i32
                - text_texture_query.height as i32
                - ortho_padding_px;
            let text_rect = Rect::new(
                x_position,
                y_position,
                text_texture_query.width,
                text_texture_query.height,
            );
            self.window
                .canvas
                .copy(&text_texture, None, Some(text_rect))
                .map_err(|e| anyhow!("{e}"))?;
        }
        if let Some(y) = y {
            let text_surface = self
                .window
                .font
                .render(y)
                .blended(Color::WHITE)
                .map_err(|e| anyhow!("{e}"))?;
            let text_surface =
                text_surface.rotate_90deg(1).map_err(|s| anyhow!("{}", s))?;
            let text_texture =
                text_surface.as_texture(&self.window.texture_creator)?;
            let text_texture_query = text_texture.query();
            let x_position = ortho_padding_px;
            let y_position = padding_px;
            let text_rect = Rect::new(
                x_position,
                y_position,
                text_texture_query.width,
                text_texture_query.height,
            );
            self.window
                .canvas
                .copy(&text_texture, None, Some(text_rect))
                .map_err(|e| anyhow!("{e}"))?;
        }

        Ok(())
    }

    fn render_axes_labels(&mut self) -> anyhow::Result<()> {
        let x = self.axis_labels.x.as_ref().map(|x| format!("{}", x));
        let y = self.axis_labels.y.as_ref().map(|y| format!("{}", y));
        self.render_axes(x.as_deref(), y.as_deref(), 20)
    }

    fn render_axes_values(&mut self) -> anyhow::Result<()> {
        let (x, y) = self.value_01();
        let x = format!("{}", x);
        let y = format!("{}", y);
        self.render_axes(Some(x.as_str()), Some(y.as_str()), 0)
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.window.canvas.set_draw_color(Color::BLACK);
        self.window.canvas.clear();
        self.render_xy()?;
        self.window.render_title(TitlePosition::Center)?;
        self.render_axes_labels()?;
        self.render_axes_values()?;
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

    pub fn value_01(&self) -> (f32, f32) {
        (
            self.state.x_px as f32 / WIDTH_PX as f32,
            self.state.y_px as f32 / HEIGHT_PX as f32,
        )
    }

    pub fn value_midi(&self) -> (u7, u7) {
        let (x, y) = self.value_01();
        (
            ((x * u7::max_value().as_int() as f32) as u8).into(),
            ((y * u7::max_value().as_int() as f32) as u8).into(),
        )
    }

    pub fn is_space_pressed(&self) -> bool {
        self.space_pressed
    }
}
