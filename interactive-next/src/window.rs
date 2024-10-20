use anyhow::anyhow;
use caw_core_next::{Sig, SigBuf};
use caw_next::player::{Player, ToF32};
use line_2d::Coord;
pub use rgb_int::Rgb24;
use sdl2::{
    pixels::Color, rect::Rect, render::Canvas, video::Window as Sdl2Window,
};
use std::{
    thread,
    time::{Duration, Instant},
};

const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / 60);
const WARMUP_FRAMES: usize = 10;

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

    pub fn sane_default(self) -> Self {
        self.line_width(4).stable(true).spread(2).scale(2.0)
    }

    pub fn build(self) -> Window {
        Window {
            title: self.title.unwrap_or_else(|| "CAW Synthesizer".to_string()),
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

struct WindowRunning {
    window: Window,
    canvas: Canvas<sdl2::video::Window>,
    event_pump: sdl2::EventPump,
    visualization_state: VisualizationState,
    scratch: Scratch,
    foreground: Color,
    background: Color,
    last_render: Instant,
}

impl WindowRunning {
    fn render_visualization(&mut self) {
        render_visualization(
            &mut self.canvas,
            &mut self.scratch,
            &mut self.visualization_state,
            self.window.width_px,
            self.window.height_px,
            self.window.stable,
            self.window.stride,
            self.window.scale,
            self.window.spread,
            self.foreground,
            self.background,
            self.window.line_width,
        );
    }

    fn render(&mut self) {
        self.render_visualization();
        self.canvas.present();
    }

    fn handle_events(&mut self) {
        for event in self.event_pump.poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::Quit { .. } => std::process::exit(0),
                _ => (),
            }
        }
    }

    fn add_samples_to_visualization<T: ToF32 + Copy>(&mut self, buf: &[T]) {
        for sample in buf {
            self.visualization_state.add_sample(sample.to_f32());
        }
    }

    fn handle_frame_if_enough_time_since_previous_frame(&mut self) {
        if self.last_render.elapsed() > FRAME_DURATION {
            self.handle_events();
            self.render();
            self.last_render = Instant::now();
        }
    }
}

#[derive(Clone)]
pub struct Window {
    title: String,
    width_px: u32,
    height_px: u32,
    stable: bool,
    stride: usize,
    scale: f32,
    spread: usize,
    line_width: u32,
    foreground: Rgb24,
    background: Rgb24,
}

impl Window {
    fn run(self) -> anyhow::Result<WindowRunning> {
        let sdl_context = sdl2::init().map_err(|e| anyhow!(e))?;
        let video_subsystem = sdl_context.video().map_err(|e| anyhow!(e))?;
        let window = video_subsystem
            .window(self.title.as_str(), self.width_px, self.height_px)
            .position_centered()
            .build()?;
        let canvas = window
            .into_canvas()
            .target_texture()
            .present_vsync()
            .build()?;
        let event_pump = sdl_context.event_pump().map_err(|e| anyhow!(e))?;
        let visualization_state = VisualizationState::new();
        let scratch = Scratch::default();
        let foreground =
            Color::RGB(self.foreground.r, self.foreground.g, self.foreground.b);
        let background =
            Color::RGB(self.background.r, self.background.g, self.background.b);
        Ok(WindowRunning {
            window: self,
            canvas,
            event_pump,
            visualization_state,
            scratch,
            foreground,
            background,
            last_render: Instant::now(),
        })
    }

    pub fn play<T, S>(&self, signal: SigBuf<S>) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: Sig<Item = T, Buf = Vec<T>>,
    {
        let player = Player::new()?;
        let mut window_running = self.clone().run()?;
        // Render a few frames to warm up. The first few frames can take longer than usual to
        // render, so warming up prevents the audio from stuttering on startup.
        for _ in 0..WARMUP_FRAMES {
            window_running.handle_frame_if_enough_time_since_previous_frame();
            thread::sleep(FRAME_DURATION);
        }
        player.play_signal_sync_callback(signal, |buf| {
            // Interleave rendering with sending samples to the sound card. Rendering needs to
            // happen on the main thread as this is a requirement of SDL, and sending samples to
            // the sound card needs to happen on the main thread as signals are not `Send`.
            window_running.add_samples_to_visualization(buf);
            window_running.handle_frame_if_enough_time_since_previous_frame();
        })?;
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
                if (sample <= 0.0) && (last >= 0.0) {
                    self.zero_cross_index = Some(self.queue.len());
                }
            }
        }
        self.queue.push(sample);
    }

    // returns a sample index that can be used to plot the sample such that it crosses 0 in the
    // middle of the screen
    fn stable_start_index(&self, min_width: usize) -> Option<usize> {
        self.zero_cross_index.and_then(|zero_cross_index| {
            if self.queue.len() - zero_cross_index < (min_width / 2) {
                None
            } else {
                if zero_cross_index >= min_width / 2 {
                    Some(zero_cross_index - (min_width / 2))
                } else {
                    for i in
                        (min_width / 2)..(self.queue.len() - (min_width / 2))
                    {
                        if self.queue[i] <= 0.0 && self.queue[i - 1] >= 0.0 {
                            return Some(i - (min_width / 2));
                        }
                    }
                    None
                }
            }
        })
    }
}

#[derive(Default)]
struct Scratch {
    coords: Vec<Coord>,
    mean_y: i32,
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
        let mut total_y = 0;
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
                total_y += coord.y;
            }
        }
        self.mean_y = total_y / self.coords.len() as i32;
    }
}

fn render_visualization(
    canvas: &mut Canvas<Sdl2Window>,
    scratch: &mut Scratch,
    visualization_state: &mut VisualizationState,
    width_px: u32,
    height_px: u32,
    stable: bool,
    stride: usize,
    scale: f32,
    spread: usize,
    foreground: Color,
    background: Color,
    line_width: u32,
) {
    if visualization_state.queue.len() >= width_px as usize {
        if stable {
            if let Some(sample_start_index) = visualization_state
                .stable_start_index((width_px as usize * stride) / spread)
            {
                scratch.update_coords(
                    &visualization_state.queue[sample_start_index..],
                    width_px,
                    height_px,
                    stride,
                    scale,
                    spread,
                );
            } else {
                scratch.update_coords(
                    &visualization_state.queue,
                    width_px,
                    height_px,
                    stride,
                    scale,
                    spread,
                );
            }
        } else {
            scratch.update_coords(
                &visualization_state.queue,
                width_px,
                height_px,
                stride,
                scale,
                spread,
            );
        };
        visualization_state.clear();
    }
    canvas.set_draw_color(background);
    canvas.clear();
    canvas.set_draw_color(foreground);
    for pair in scratch.coords.windows(2) {
        for Coord { x, y } in line_2d::coords_between(pair[0], pair[1]) {
            let rect = Rect::new(x, y, line_width, line_width);
            let _ = canvas.fill_rect(rect);
        }
    }
}
