use anyhow::anyhow;
use caw_window_utils::{
    font::{Font, load_font},
    persistent::{PersistentData, WindowPosition},
};
use sdl2::{
    EventPump,
    event::{Event, WindowEvent},
    pixels::Color,
    rect::Rect,
    render::{Canvas, TextureCreator},
    video::{Window as SdlWindow, WindowContext},
};
use std::{
    thread,
    time::{Duration, Instant},
};

const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / 60);

pub enum TitlePosition {
    Center,
    CenterBottom,
}

pub struct Window {
    pub canvas: Canvas<SdlWindow>,
    pub event_pump: EventPump,
    pub font: Font<'static, 'static>,
    pub texture_creator: TextureCreator<WindowContext>,
    pub title: Option<String>,
    pub prev_tick_complete: Instant,
    width_px: u32,
    height_px: u32,
}

impl Window {
    pub fn new(
        title: Option<&str>,
        width_px: u32,
        height_px: u32,
    ) -> anyhow::Result<Self> {
        let sdl_context = sdl2::init().map_err(|e| anyhow!(e))?;
        let video_subsystem = sdl_context.video().map_err(|e| anyhow!(e))?;
        let mut window_builder =
            video_subsystem.window("", width_px, height_px);
        window_builder.always_on_top();
        if let Some(title) = title {
            if let Some(WindowPosition { x, y }) = WindowPosition::load_(title)
            {
                window_builder.position(x, y);
            }
        }
        let window = window_builder.build()?;
        let canvas = window
            .into_canvas()
            .target_texture()
            .present_vsync()
            .build()?;
        let texture_creator = canvas.texture_creator();
        let event_pump = sdl_context.event_pump().map_err(|e| anyhow!(e))?;
        Ok(Self {
            canvas,
            event_pump,
            font: load_font(16)?,
            texture_creator,
            title: title.map(|t| t.to_string()),
            prev_tick_complete: Instant::now(),
            width_px,
            height_px,
        })
    }

    pub fn wait_until_next_frame(&self) {
        if let Some(period_to_sleep) = (self.prev_tick_complete
            + FRAME_DURATION)
            .checked_duration_since(Instant::now())
        {
            thread::sleep(period_to_sleep);
        }
    }

    pub fn render_title(
        &mut self,
        position: TitlePosition,
    ) -> anyhow::Result<()> {
        if let Some(title) = self.title.as_ref() {
            let text_surface = self
                .font
                .render(title.as_str())
                .blended(Color::WHITE)
                .map_err(|e| anyhow!("{e}"))?;
            let text_texture =
                text_surface.as_texture(&self.texture_creator)?;
            let (canvas_width, canvas_height) =
                self.canvas.output_size().map_err(|e| anyhow!("{e}"))?;
            let text_texture_query = text_texture.query();
            let value_space_px = 20;
            let x_position =
                (canvas_width as i32 - text_texture_query.width as i32) / 2;
            let y_position = match position {
                TitlePosition::Center => {
                    (canvas_height as i32 - text_texture_query.height as i32)
                        / 2
                }
                TitlePosition::CenterBottom => {
                    canvas_height as i32
                        - text_texture_query.height as i32
                        - value_space_px
                }
            };
            let text_rect = Rect::new(
                x_position,
                y_position,
                text_texture_query.width,
                text_texture_query.height,
            );
            self.canvas
                .copy(&text_texture, None, Some(text_rect))
                .map_err(|e| anyhow!("{e}"))?;
        }
        Ok(())
    }

    pub fn width_px(&self) -> u32 {
        self.width_px
    }

    pub fn height_px(&self) -> u32 {
        self.height_px
    }

    pub fn handle_event_common(event: Event, title: Option<&String>) {
        match event {
            Event::Quit { .. } => std::process::exit(0),
            Event::Window {
                win_event: WindowEvent::Moved(x, y),
                ..
            } => {
                if let Some(title) = title {
                    (WindowPosition { x, y }).save_(title);
                }
            }
            _ => (),
        }
    }
}
