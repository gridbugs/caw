use crate::window::Window;
use anyhow::anyhow;
use line_2d::Coord;
use midly::num::u7;
use sdl2::{mouse::MouseButton, pixels::Color, rect::Rect};
use std::time::Instant;

const WIDTH_PX: u32 = 128;
const HEIGHT_PX: u32 = 128;

const RANGE_RADS: f32 = (std::f32::consts::PI * 3.0) / 2.0;

pub struct Knob {
    window: Window,
    value_01: f32,
    sensitivity: f32,
}

impl Knob {
    pub fn new(
        title: Option<&str>,
        initial_value_01: f32,
        sensitivity: f32,
    ) -> anyhow::Result<Self> {
        let window = Window::new(title, WIDTH_PX, HEIGHT_PX)?;
        Ok(Self {
            window,
            value_01: initial_value_01.clamp(0., 1.),
            sensitivity,
        })
    }

    fn handle_events(&mut self) {
        for event in self.window.event_pump.poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::Quit { .. } => std::process::exit(0),
                Event::MouseWheel { precise_y, .. } => {
                    let multiplier = 0.1;
                    self.value_01 = (self.value_01
                        + (precise_y * multiplier * self.sensitivity))
                        .clamp(0., 1.);
                }
                Event::MouseMotion {
                    mousestate, yrel, ..
                } => {
                    if mousestate.is_mouse_button_pressed(MouseButton::Left) {
                        let multiplier = -0.05;
                        self.value_01 = (self.value_01
                            + (yrel as f32 * multiplier * self.sensitivity))
                            .clamp(0., 1.);
                    }
                }
                _ => (),
            }
        }
    }

    fn render_value(&mut self) -> anyhow::Result<()> {
        let text_surface = self
            .window
            .font
            .render(format!("{}", self.value_01).as_str())
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
        let relative_angle_rads = RANGE_RADS * self.value_01;
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
        self.window.render_title()?;
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
        self.value_01
    }

    pub fn value_midi(&self) -> u7 {
        ((self.value_01() * u7::max_value().as_int() as f32) as u8).into()
    }
}
