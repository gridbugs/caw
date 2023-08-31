use crate::{
    signal::Signal,
    signal_player::{SignalPlayer, ToF32},
};
use anyhow::anyhow;
use line_2d::Coord;
pub use rgb_int::Rgb24;
use sdl2::{pixels::Color, rect::Rect, render::Canvas, video::Window as Sdl2Window};
use std::{
    thread,
    time::{Duration, Instant},
};

const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / 60);

pub struct WindowBuilder {
    title: Option<String>,
    width_px: Option<u32>,
    height_px: Option<u32>,
    stable: Option<bool>,
    stride: Option<usize>,
    scale: Option<f32>,
    spread: Option<usize>,
    line_width: Option<u32>,
    foreground: Option<Rgb24>,
    background: Option<Rgb24>,
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {
            title: None,
            width_px: None,
            height_px: None,
            stable: None,
            stride: None,
            scale: None,
            spread: None,
            line_width: None,
            foreground: None,
            background: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn width_px(mut self, width_px: u32) -> Self {
        self.width_px = Some(width_px.into());
        self
    }

    pub fn height_px(mut self, height_px: u32) -> Self {
        self.height_px = Some(height_px.into());
        self
    }

    pub fn stable(mut self, stable: bool) -> Self {
        self.stable = Some(stable.into());
        self
    }

    pub fn stride(mut self, stride: usize) -> Self {
        self.stride = Some(stride.into());
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = Some(scale.into());
        self
    }

    pub fn spread(mut self, spread: usize) -> Self {
        self.spread = Some(spread.into());
        self
    }

    pub fn line_width(mut self, line_width: u32) -> Self {
        self.line_width = Some(line_width);
        self
    }

    pub fn foreground(mut self, foreground: Rgb24) -> Self {
        self.foreground = Some(foreground.into());
        self
    }

    pub fn background(mut self, background: Rgb24) -> Self {
        self.background = Some(background.into());
        self
    }

    pub fn build(self) -> Window {
        Window {
            title: self.title.unwrap_or_else(|| "Ibis".to_string()),
            width_px: self.width_px.unwrap_or(960),
            height_px: self.height_px.unwrap_or(720),
            stable: self.stable.unwrap_or(false),
            stride: self.stride.unwrap_or(1),
            scale: self.scale.unwrap_or(1.0),
            spread: self.spread.unwrap_or(1),
            line_width: self.line_width.unwrap_or(1),
            foreground: self.foreground.unwrap_or_else(|| Rgb24::new_grey(255)),
            background: self.background.unwrap_or_else(|| Rgb24::new_grey(0)),
        }
    }
}

pub struct Window {
    pub title: String,
    pub width_px: u32,
    pub height_px: u32,
    pub stable: bool,
    pub stride: usize,
    pub scale: f32,
    pub spread: usize,
    pub line_width: u32,
    pub foreground: Rgb24,
    pub background: Rgb24,
}

impl Window {
    pub fn play<T: Clone + ToF32 + 'static>(
        &self,
        buffered_signal: &mut Signal<T>,
    ) -> anyhow::Result<()> {
        let sdl_context = sdl2::init().map_err(|e| anyhow!(e))?;
        let video_subsystem = sdl_context.video().map_err(|e| anyhow!(e))?;
        let window = video_subsystem
            .window(self.title.as_str(), self.width_px, self.height_px)
            .position_centered()
            .build()?;
        let mut canvas = window
            .into_canvas()
            .target_texture()
            .present_vsync()
            .build()?;
        // Skip this many frames to prevent choppy audio on startup.
        let mut warmup_frames = 15;
        let mut event_pump = sdl_context.event_pump().map_err(|e| anyhow!(e))?;
        let mut signal_player = SignalPlayer::new()?;
        let mut visualization_state = VisualizationState::new();
        let mut scratch = Scratch::default();
        let foreground = Color::RGB(self.foreground.r, self.foreground.g, self.foreground.b);
        let background = Color::RGB(self.background.r, self.background.g, self.background.b);
        'running: loop {
            let frame_start = Instant::now();
            for event in event_pump.poll_iter() {
                use sdl2::event::Event;
                match event {
                    Event::Quit { .. } => break 'running,
                    _ => (),
                }
            }
            if warmup_frames > 0 {
                warmup_frames -= 1;
            } else {
                visualization_state.clear();
                signal_player.send_signal_with_callback(buffered_signal, |sample| {
                    visualization_state.add_sample(sample);
                });
            }
            render_visualization(
                &mut canvas,
                &mut scratch,
                &visualization_state,
                self.width_px,
                self.height_px,
                self.stable,
                self.stride,
                self.scale,
                self.spread,
                foreground,
                background,
                self.line_width,
            )?;
            canvas.present();
            let since_frame_start = frame_start.elapsed();
            if let Some(until_next_frame) = FRAME_DURATION.checked_sub(since_frame_start) {
                thread::sleep(until_next_frame);
            }
        }
        Ok(())
    }

    pub fn builder() -> WindowBuilder {
        WindowBuilder::new()
    }
}

struct VisualizationState {
    queue: Vec<f32>,
    zero_cross_index: Option<usize>,
}

impl VisualizationState {
    fn new() -> Self {
        Self {
            queue: Vec::new(),
            zero_cross_index: None,
        }
    }

    fn clear(&mut self) {
        self.queue.clear();
        self.zero_cross_index = None;
    }

    fn add_sample(&mut self, sample: f32) {
        if self.zero_cross_index.is_none() {
            if let Some(&last) = self.queue.last() {
                if (sample > 0.0) && (last < 0.0) {
                    self.zero_cross_index = Some(self.queue.len());
                }
            }
        }
        self.queue.push(sample);
    }

    fn stable_start_index(&self, min_width: usize) -> Option<usize> {
        self.zero_cross_index.and_then(|zero_cross_index| {
            if self.queue.len() - zero_cross_index < min_width {
                None
            } else {
                Some(zero_cross_index)
            }
        })
    }
}

#[derive(Default)]
struct Scratch {
    coords: Vec<Coord>,
}

impl Scratch {
    fn update_coords(
        &mut self,
        samples: &[f32],
        width_px: u32,
        height_px: u32,
        stride: usize,
        scale: f32,
        spread: usize,
    ) {
        self.coords.clear();
        let height_px = height_px as f32;
        let y_px_mid = height_px * 0.5;
        let sample_mult = scale * height_px * 0.25;
        for x_px in 0..(width_px as usize / spread) {
            let sample_index = x_px * stride;
            if let Some(sample) = samples.get(sample_index) {
                let y_px = y_px_mid - (sample * sample_mult);
                let coord = Coord {
                    x: (x_px * spread) as i32,
                    y: y_px as i32,
                };
                self.coords.push(coord);
            }
        }
    }
}

fn render_visualization(
    canvas: &mut Canvas<Sdl2Window>,
    scratch: &mut Scratch,
    visualization_state: &VisualizationState,
    width_px: u32,
    height_px: u32,
    stable: bool,
    stride: usize,
    scale: f32,
    spread: usize,
    foreground: Color,
    background: Color,
    line_width: u32,
) -> anyhow::Result<()> {
    if stable {
        if let Some(sample_start_index) =
            visualization_state.stable_start_index((width_px as usize * stride) / spread)
        {
            scratch.update_coords(
                &visualization_state.queue[sample_start_index..],
                width_px,
                height_px,
                stride,
                scale,
                spread,
            );
        }
    } else {
        if !visualization_state.queue.is_empty() {
            scratch.update_coords(
                &visualization_state.queue,
                width_px,
                height_px,
                stride,
                scale,
                spread,
            );
        }
    };
    canvas.set_draw_color(background);
    canvas.clear();
    canvas.set_draw_color(foreground);
    for pair in scratch.coords.windows(2) {
        for Coord { x, y } in line_2d::coords_between(pair[0], pair[1]) {
            let rect = Rect::new(x, y, line_width, line_width);
            canvas.fill_rect(rect).map_err(|e| anyhow!(e))?;
        }
    }
    Ok(())
}
