use anyhow::anyhow;
use lazy_static::lazy_static;
use sdl2::{
    EventPump,
    pixels::Color,
    rect::Rect,
    render::{Canvas, TextureCreator},
    rwops::RWops,
    ttf::{Font, Sdl2TtfContext},
    video::{Window as SdlWindow, WindowContext},
};
use std::{
    thread,
    time::{Duration, Instant},
};

const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / 60);

lazy_static! {
    static ref TTF_CONTEXT: Result<Sdl2TtfContext, String> = sdl2::ttf::init();
}

fn load_font() -> anyhow::Result<Font<'static, 'static>> {
    let font_data = include_bytes!("inconsolata.regular.ttf");
    let pt_size = 16;
    let rwops = RWops::from_bytes(font_data).map_err(|e| anyhow!("{e}"))?;
    let ttf_context = TTF_CONTEXT.as_ref().map_err(|e| anyhow!("{e}"))?;
    ttf_context
        .load_font_from_rwops(rwops, pt_size)
        .map_err(|e| anyhow!(e))
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
        let window = video_subsystem
            .window("", width_px, height_px)
            .always_on_top()
            .build()?;
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
            font: load_font()?,
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

    pub fn render_title(&mut self) -> anyhow::Result<()> {
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
            // Render the title centred at the bottom of the window.
            let text_rect = Rect::new(
                (canvas_width as i32 - text_texture_query.width as i32) / 2,
                canvas_height as i32
                    - text_texture_query.height as i32
                    - value_space_px,
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
}
