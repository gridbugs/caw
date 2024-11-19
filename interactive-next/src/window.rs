use crate::input::{Input, InputState};
use anyhow::anyhow;
use caw_core_next::SigT;
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

#[derive(Debug, Clone, Copy)]
pub enum Visualization {
    Oscilloscope,
    StereoOscillographics,
}

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
    visualization: Option<Visualization>,
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
            visualization: None,
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

    pub fn visualization(mut self, visualization: Visualization) -> Self {
        self.visualization = Some(visualization);
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
            visualization: self
                .visualization
                .unwrap_or_else(|| Visualization::Oscilloscope),
            input_state: InputState::new(),
        }
    }
}

struct WindowRunning {
    window: Window,
    canvas: Canvas<sdl2::video::Window>,
    event_pump: sdl2::EventPump,
    oscilloscope_state: OscilloscopeState,
    oscilloscope_scratch: OscilloscopeScratch,
    stereo_oscillographics_state: StereoOscillographicsState,
    foreground: Color,
    background: Color,
    last_render: Instant,
}

impl WindowRunning {
    fn render_visualization(&mut self) {
        match self.window.visualization {
            Visualization::Oscilloscope => self.oscilloscope_state.render(
                &mut self.canvas,
                &mut self.oscilloscope_scratch,
                self.window.width_px,
                self.window.height_px,
                self.window.stable,
                self.window.stride,
                self.window.scale,
                self.window.spread,
                self.foreground,
                self.background,
                self.window.line_width,
            ),
            Visualization::StereoOscillographics => {
                self.stereo_oscillographics_state.render(
                    &mut self.canvas,
                    self.window.width_px,
                    self.window.height_px,
                    self.window.foreground,
                    self.window.background,
                    self.window.line_width,
                    self.window.stable,
                    self.window.stride,
                    self.window.scale,
                )
            }
        }
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
                Event::KeyDown {
                    scancode: Some(scancode),
                    repeat: false,
                    ..
                } => self.window.input_state.set_key(scancode, true),
                Event::KeyUp {
                    scancode: Some(scancode),
                    repeat: false,
                    ..
                } => self.window.input_state.set_key(scancode, false),
                Event::MouseMotion { x, y, .. } => {
                    self.window.input_state.set_mouse_position(
                        x as f32 / self.window.width_px as f32,
                        y as f32 / self.window.height_px as f32,
                    )
                }
                Event::MouseButtonDown { mouse_btn, .. } => {
                    self.window.input_state.set_mouse_button(mouse_btn, true)
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    self.window.input_state.set_mouse_button(mouse_btn, false)
                }

                _ => (),
            }
        }
    }

    fn add_samples_to_oscilloscope_state<T: ToF32 + Copy>(
        &mut self,
        buf: &[T],
    ) {
        for sample in buf {
            self.oscilloscope_state.add_sample(sample.to_f32());
        }
    }

    fn add_samples_to_stereo_oscillographics_state<
        TL: ToF32 + Copy,
        TR: ToF32 + Copy,
    >(
        &mut self,
        buf_left: &[TL],
        buf_right: &[TR],
    ) {
        for (left, right) in buf_left.into_iter().zip(buf_right.into_iter()) {
            self.stereo_oscillographics_state
                .add_sample(left.to_f32(), right.to_f32());
        }
    }

    fn handle_frame_if_enough_time_since_previous_frame(&mut self) {
        self.handle_events();
        if self.last_render.elapsed() > FRAME_DURATION {
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
    visualization: Visualization,
    input_state: InputState,
}

impl Window {
    fn run(self) -> anyhow::Result<WindowRunning> {
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
        canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
        let event_pump = sdl_context.event_pump().map_err(|e| anyhow!(e))?;
        let oscilloscope_state = OscilloscopeState::new();
        let oscilloscope_scratch = OscilloscopeScratch::default();
        let stereo_oscillographics_state = StereoOscillographicsState::new();
        let foreground =
            Color::RGB(self.foreground.r, self.foreground.g, self.foreground.b);
        let background =
            Color::RGB(self.background.r, self.background.g, self.background.b);
        Ok(WindowRunning {
            window: self,
            canvas,
            event_pump,
            oscilloscope_state,
            oscilloscope_scratch,
            stereo_oscillographics_state,
            foreground,
            background,
            last_render: Instant::now(),
        })
    }

    pub fn input(&self) -> Input {
        self.input_state.input()
    }

    pub fn play_mono<T, S>(&self, sig: S) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: SigT<Item = T>,
    {
        let player = Player::new()?;
        let mut window_running = self.clone().run()?;
        // Render a few frames to warm up. The first few frames can take longer than usual to
        // render, so warming up prevents the audio from stuttering on startup.
        for _ in 0..WARMUP_FRAMES {
            window_running.handle_frame_if_enough_time_since_previous_frame();
            thread::sleep(FRAME_DURATION);
        }
        player.play_signal_sync_mono_callback(sig, |buf| {
            // Interleave rendering with sending samples to the sound card. Rendering needs to
            // happen on the main thread as this is a requirement of SDL, and sending samples to
            // the sound card needs to happen on the main thread as signals are not `Send`.
            window_running.add_samples_to_oscilloscope_state(buf);
            window_running.handle_frame_if_enough_time_since_previous_frame();
        })?;
        Ok(())
    }

    pub fn play_stereo<TL, TR, SL, SR>(
        &self,
        sig_left: SL,
        sig_right: SR,
    ) -> anyhow::Result<()>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
        SL: SigT<Item = TL>,
        SR: SigT<Item = TR>,
    {
        let player = Player::new()?;
        let mut window_running = self.clone().run()?;
        // Render a few frames to warm up. The first few frames can take longer than usual to
        // render, so warming up prevents the audio from stuttering on startup.
        for _ in 0..WARMUP_FRAMES {
            window_running.handle_frame_if_enough_time_since_previous_frame();
            thread::sleep(FRAME_DURATION);
        }
        player.play_signal_sync_stereo_callback(
            sig_left,
            sig_right,
            |buf_left, buf_right| {
                // Interleave rendering with sending samples to the sound card. Rendering needs to
                // happen on the main thread as this is a requirement of SDL, and sending samples to
                // the sound card needs to happen on the main thread as signals are not `Send`.

                match self.visualization {
                    Visualization::Oscilloscope => window_running
                        .add_samples_to_oscilloscope_state(buf_left),
                    Visualization::StereoOscillographics => window_running
                        .add_samples_to_stereo_oscillographics_state(
                            buf_left, buf_right,
                        ),
                }
                window_running
                    .handle_frame_if_enough_time_since_previous_frame();
            },
        )?;
        Ok(())
    }

    pub fn builder() -> WindowBuilder {
        WindowBuilder::new()
    }
}

struct OscilloscopeState {
    queue: Vec<f32>,
    zero_cross_index: Option<usize>,
}

impl OscilloscopeState {
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

    fn render(
        &mut self,
        canvas: &mut Canvas<Sdl2Window>,
        oscilloscope_scratch: &mut OscilloscopeScratch,
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
        if self.queue.len() >= width_px as usize {
            if stable {
                if let Some(sample_start_index) = self
                    .stable_start_index((width_px as usize * stride) / spread)
                {
                    oscilloscope_scratch.update_coords(
                        &self.queue[sample_start_index..],
                        width_px,
                        height_px,
                        stride,
                        scale,
                        spread,
                    );
                } else {
                    oscilloscope_scratch.update_coords(
                        &self.queue,
                        width_px,
                        height_px,
                        stride,
                        scale,
                        spread,
                    );
                }
            } else {
                oscilloscope_scratch.update_coords(
                    &self.queue,
                    width_px,
                    height_px,
                    stride,
                    scale,
                    spread,
                );
            };
            self.clear();
        }
        canvas.set_draw_color(background);
        canvas.clear();
        canvas.set_draw_color(foreground);
        for pair in oscilloscope_scratch.coords.windows(2) {
            for Coord { x, y } in line_2d::coords_between(pair[0], pair[1]) {
                let rect = Rect::new(x, y, line_width, line_width);
                let _ = canvas.fill_rect(rect);
            }
        }
    }
}

#[derive(Default)]
struct OscilloscopeScratch {
    coords: Vec<Coord>,
    mean_y: i32,
}

impl OscilloscopeScratch {
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

struct StereoOscillographicsState {
    samples: Vec<(f32, f32)>,
}

impl StereoOscillographicsState {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
        }
    }

    fn add_sample(&mut self, left: f32, right: f32) {
        self.samples.push((left, right));
    }

    fn render(
        &mut self,
        canvas: &mut Canvas<Sdl2Window>,
        width_px: u32,
        height_px: u32,
        foreground: Rgb24,
        background: Rgb24,
        line_width: u32,
        stable: bool,
        stride: usize,
        scale: f32,
    ) {
        canvas.set_draw_color(Color::RGB(
            background.r,
            background.g,
            background.b,
        ));
        canvas.clear();
        let start = if stable {
            let mut start = 0;
            for (i, w) in self.samples.windows(2).enumerate() {
                if w[0].0 <= 0.0 && w[0].0 >= 0.0 {
                    start = i;
                    break;
                }
            }
            start
        } else {
            0
        };
        if let Some(&first) = self.samples.get(start) {
            // Scale points so a value of 1 is the edge of the window in the shortest dimension
            // (assuming the scale argument is 1).
            let scale = scale * width_px.min(height_px) as f32 / 2.0;
            let offset = Coord {
                x: width_px as i32 / 2,
                y: height_px as i32 / 2,
            };
            let transform = |(x, y): (f32, f32)| {
                Coord {
                    x: (x * scale) as i32,
                    y: (y * scale) as i32,
                } + offset
            };
            let mut prev = transform(first);
            let mut alpha = 0.0;
            let alpha_step =
                1.0 / ((self.samples.len() - start) / stride) as f32;
            for chunk in self.samples[(start + 1)..].chunks(stride) {
                let coord = transform(chunk[0]);
                let foreground = Color::RGBA(
                    foreground.r,
                    foreground.g,
                    foreground.b,
                    (255.0 * alpha) as u8,
                );
                alpha += alpha_step;
                canvas.set_draw_color(foreground);
                for Coord { x, y } in line_2d::coords_between(prev, coord) {
                    let rect = Rect::new(x, y, line_width, line_width);
                    let _ = canvas.fill_rect(rect);
                }
                prev = coord;
            }
            self.samples.clear();
        }
    }
}
