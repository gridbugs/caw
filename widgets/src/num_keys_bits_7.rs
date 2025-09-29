use crate::window::{TitlePosition, Window};
use midly::num::u7;
use sdl2::{event::Event, keyboard::Scancode, pixels::Color};
use std::time::Instant;

const WIDTH_PX: u32 = 128;
const HEIGHT_PX: u32 = 128;

pub struct NumKeysBits7 {
    window: Window,
    space_pressed: bool,
    state: u8,
}

fn scancode_to_mask(scancode: Scancode) -> Option<u8> {
    let mask = match scancode {
        Scancode::Grave | Scancode::Num0 | Scancode::Kp0 => {
            // Include the Grave key for 0 since it's to the left of the 1 key
            1 << 0
        }
        Scancode::Num1 | Scancode::Kp1 => 1 << 1,
        Scancode::Num2 | Scancode::Kp2 => 1 << 2,
        Scancode::Num3 | Scancode::Kp3 => 1 << 3,
        Scancode::Num4 | Scancode::Kp4 => 1 << 4,
        Scancode::Num5 | Scancode::Kp5 => 1 << 5,
        Scancode::Num6 | Scancode::Kp6 => 1 << 6,
        _ => return None,
    };
    Some(mask)
}

impl NumKeysBits7 {
    pub fn new(title: Option<&str>) -> anyhow::Result<Self> {
        let window = Window::new(title, WIDTH_PX, HEIGHT_PX)?;
        Ok(Self {
            window,
            space_pressed: false,
            state: 0,
        })
    }

    fn handle_events(&mut self) {
        for event in self.window.event_pump.poll_iter() {
            Window::handle_event_common(
                event.clone(),
                self.window.title.as_ref(),
            );
            match event {
                Event::KeyDown { scancode, .. } => match scancode {
                    Some(Scancode::Space) => self.space_pressed = true,
                    Some(scancode) => {
                        if let Some(mask) = scancode_to_mask(scancode) {
                            self.state |= mask;
                        }
                    }
                    _ => (),
                },
                Event::KeyUp { scancode, .. } => match scancode {
                    Some(Scancode::Space) => self.space_pressed = false,
                    Some(scancode) => {
                        if let Some(mask) = scancode_to_mask(scancode) {
                            self.state &= !mask;
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        }
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.window.canvas.set_draw_color(Color::BLACK);
        self.window.canvas.clear();
        self.window.render_title(TitlePosition::CenterBottom)?;
        self.window.canvas.present();
        Ok(())
    }

    fn update(&mut self) -> anyhow::Result<()> {
        self.handle_events();
        self.render()?;
        Ok(())
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        self.window.wait_until_next_frame();
        self.update()?;
        self.window.prev_tick_complete = Instant::now();
        Ok(())
    }

    pub fn value(&self) -> u7 {
        u7::from_int_lossy(self.state)
    }

    pub fn is_space_pressed(&self) -> bool {
        self.space_pressed
    }
}
