use crate::window::Window;
use anyhow::anyhow;
use midly::num::u7;
use sdl2::{mouse::MouseButton, pixels::Color, rect::Rect};
use std::time::Instant;

const WIDTH_PX: u32 = 128;
const HEIGHT_PX: u32 = 128;

pub struct Xy {
    window: Window,
    x_px: u32,
    y_px: u32,
}

impl Xy {
    pub fn new(title: Option<&str>) -> anyhow::Result<Self> {
        let window = Window::new(title, WIDTH_PX, HEIGHT_PX)?;
        Ok(Self {
            window,
            x_px: WIDTH_PX / 2,
            y_px: HEIGHT_PX / 2,
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
                Event::MouseButtonDown { x, y, .. } => {
                    self.x_px = x.clamp(0, WIDTH_PX as i32) as u32;
                    self.y_px = y.clamp(0, HEIGHT_PX as i32) as u32;
                }
                Event::MouseMotion {
                    mousestate, x, y, ..
                } => {
                    if mousestate.is_mouse_button_pressed(MouseButton::Left) {
                        self.x_px = x.clamp(0, WIDTH_PX as i32) as u32;
                        self.y_px = y.clamp(0, HEIGHT_PX as i32) as u32;
                    }
                }
                _ => (),
            }
        }
    }

    fn render_xy(&mut self) -> anyhow::Result<()> {
        self.window.canvas.set_draw_color(Color::GREY);
        let line_width = 2;
        let rect_horizontal = Rect::new(
            0,
            self.y_px as i32 - line_width as i32 / 2,
            self.window.width_px(),
            line_width,
        );
        let rect_vertical = Rect::new(
            self.x_px as i32 - line_width as i32 / 2,
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

    fn render(&mut self) -> anyhow::Result<()> {
        self.window.canvas.set_draw_color(Color::BLACK);
        self.window.canvas.clear();
        self.render_xy()?;
        self.window.render_title()?;
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
            self.x_px as f32 / WIDTH_PX as f32,
            self.y_px as f32 / HEIGHT_PX as f32,
        )
    }

    pub fn value_midi(&self) -> (u7, u7) {
        let (x, y) = self.value_01();
        (
            ((x * u7::max_value().as_int() as f32) as u8).into(),
            ((y * u7::max_value().as_int() as f32) as u8).into(),
        )
    }
}
