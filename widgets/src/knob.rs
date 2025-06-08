use anyhow::anyhow;
use lazy_static::lazy_static;
use line_2d::Coord;
use midly::num::u7;
use sdl2::{
    EventPump,
    mouse::MouseButton,
    pixels::Color,
    rect::Rect,
    render::{Canvas, TextureCreator},
    rwops::RWops,
    ttf::{Font, InitError, Sdl2TtfContext},
    video::{Window, WindowContext},
};
use std::{
    thread,
    time::{Duration, Instant},
};

lazy_static! {
    static ref TTF_CONTEXT: Result<Sdl2TtfContext, InitError> =
        sdl2::ttf::init();
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

const WIDTH_PX: u32 = 128;
const HEIGHT_PX: u32 = 128;

const RANGE_RADS: f32 = (std::f32::consts::PI * 3.0) / 2.0;

const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / 60);

pub struct Knob {
    title: Option<String>,
    canvas: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
    event_pump: EventPump,
    prev_tick_complete: Instant,
    font: Font<'static, 'static>,
    value_01: f32,
    sensitivity: f32,
}

impl Knob {
    pub fn new(
        title: Option<&str>,
        initial_value_01: f32,
        sensitivity: f32,
    ) -> anyhow::Result<Self> {
        let sdl_context = sdl2::init().map_err(|e| anyhow!(e))?;
        let video_subsystem = sdl_context.video().map_err(|e| anyhow!(e))?;
        let window = video_subsystem
            .window("", WIDTH_PX, HEIGHT_PX)
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
            title: title.map(|t| t.to_string()),
            canvas,
            texture_creator,
            event_pump,
            prev_tick_complete: Instant::now(),
            font: load_font()?,
            value_01: initial_value_01.clamp(0., 1.),
            sensitivity,
        })
    }

    fn wait_until_next_frame(&self) {
        if let Some(period_to_sleep) = (self.prev_tick_complete
            + FRAME_DURATION)
            .checked_duration_since(Instant::now())
        {
            thread::sleep(period_to_sleep);
        }
    }

    fn handle_events(&mut self) {
        for event in self.event_pump.poll_iter() {
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

    fn render_title(&mut self) -> anyhow::Result<()> {
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

    fn render_value(&mut self) -> anyhow::Result<()> {
        let text_surface = self
            .font
            .render(format!("{}", self.value_01).as_str())
            .blended(Color::WHITE)
            .map_err(|e| anyhow!("{e}"))?;
        let text_texture = text_surface.as_texture(&self.texture_creator)?;
        let (canvas_width, canvas_height) =
            self.canvas.output_size().map_err(|e| anyhow!("{e}"))?;
        let text_texture_query = text_texture.query();
        // Render the title centred at the bottom of the window.
        let text_rect = Rect::new(
            (canvas_width as i32 - text_texture_query.width as i32) / 2,
            canvas_height as i32 - text_texture_query.height as i32,
            text_texture_query.width,
            text_texture_query.height,
        );
        self.canvas
            .copy(&text_texture, None, Some(text_rect))
            .map_err(|e| anyhow!("{e}"))?;
        Ok(())
    }

    fn render_knob(&mut self) -> anyhow::Result<()> {
        let (available_width, canvas_height) =
            self.canvas.output_size().map_err(|e| anyhow!("{e}"))?;
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
        self.canvas.set_draw_color(Color::WHITE);
        for Coord { x, y } in line_2d::coords_between(centre, end) {
            let rect = Rect::new(
                x - (line_width as i32 / 2),
                y - (line_width as i32 / 2),
                line_width,
                line_width,
            );
            self.canvas.fill_rect(rect).map_err(|e| anyhow!("{e}"))?;
        }
        Ok(())
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();
        self.render_title()?;
        self.render_value()?;
        self.render_knob()?;
        self.canvas.present();
        Ok(())
    }

    fn update(&mut self) -> anyhow::Result<()> {
        self.handle_events();
        self.render()?;
        Ok(())
    }

    /// Waits until the next frame, then handles events and redraws the widget
    pub fn tick(&mut self) -> anyhow::Result<()> {
        self.wait_until_next_frame();
        self.update()?;
        self.prev_tick_complete = Instant::now();
        Ok(())
    }

    pub fn value_01(&self) -> f32 {
        self.value_01
    }

    pub fn value_midi(&self) -> u7 {
        ((self.value_01() * u7::max_value().as_int() as f32) as u8).into()
    }
}
