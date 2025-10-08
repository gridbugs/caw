use crate::window::{TitlePosition, Window};
use anyhow::anyhow;
use sdl2::{keyboard::Scancode, pixels::Color, rect::Rect};
use std::time::Instant;

const WIDTH_PX: u32 = 128;
const HEIGHT_PX: u32 = 128;

pub struct Button {
    window: Window,
    pressed: bool,
    space_pressed: bool,
}

impl Button {
    pub fn new(title: Option<&str>) -> anyhow::Result<Self> {
        let window = Window::new(title, WIDTH_PX, HEIGHT_PX)?;
        Ok(Self {
            window,
            pressed: false,
            space_pressed: false,
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
                Event::MouseButtonDown { .. } => {
                    self.pressed = true;
                }
                Event::MouseButtonUp { .. } => {
                    self.pressed = false;
                }
                Event::KeyDown {
                    scancode: Some(Scancode::Space),
                    ..
                } => {
                    self.space_pressed = true;
                }
                Event::KeyUp {
                    scancode: Some(Scancode::Space),
                    ..
                } => {
                    self.space_pressed = false;
                }
                _ => (),
            }
        }
    }

    fn render_button(&mut self) -> anyhow::Result<()> {
        let padding = 10;
        let rect = Rect::new(
            padding,
            padding,
            self.window.width_px() - (2 * padding as u32),
            self.window.height_px() - (2 * padding as u32) - 40,
        );
        if self.pressed {
            self.window.canvas.set_draw_color(Color::WHITE);
            self.window
                .canvas
                .fill_rect(rect)
                .map_err(|e| anyhow!("{e}"))?;
        }
        self.window.canvas.set_draw_color(Color::GREY);
        self.window
            .canvas
            .draw_rect(rect)
            .map_err(|e| anyhow!("{e}"))?;
        Ok(())
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.window.canvas.set_draw_color(Color::BLACK);
        self.window.canvas.clear();
        self.window.render_title(TitlePosition::CenterBottom)?;
        self.render_button()?;
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
        self.pressed
    }

    pub fn is_space_pressed(&self) -> bool {
        self.space_pressed
    }
}
