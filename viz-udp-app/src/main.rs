/// A udp client which receives visualization data from a corresponding udp server and renders
/// visualizations in a graphical window.
use anyhow::anyhow;
use caw_persist::PersistData;
use caw_viz_udp_app_lib::{blink, oscilloscope};
use caw_window_utils::{
    font::load_font,
    persist::{WindowPosition, WindowSize},
};
use clap::{Parser, Subcommand, ValueEnum};
use line_2d::Coord;
use rgb_int::Rgb24;
use sdl2::{
    EventPump,
    event::{Event, WindowEvent},
    keyboard::Scancode,
    pixels::Color,
    rect::Rect,
    render::Canvas,
    video::Window,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Serialize, Deserialize, ValueEnum, Clone, Copy, Debug)]
pub enum OscilloscopeStyle {
    Xy,
    TimeDomain,
    TimeDomainStereo,
}

#[derive(Parser)]
struct OscilloscopeCommand {
    #[arg(long, default_value_t = 640)]
    width: u32,
    #[arg(long, default_value_t = 480)]
    height: u32,
    #[arg(long, default_value_t = 500.0)]
    scale: f32,
    #[arg(long, default_value_t = 5000)]
    max_num_samples: usize,
    #[arg(long, default_value_t = 1)]
    line_width: u32,
    #[arg(long, default_value_t = 255)]
    alpha_scale: u8,
    #[arg(long, short, default_value_t = 0)]
    red: u8,
    #[arg(long, short, default_value_t = 255)]
    green: u8,
    #[arg(long, short, default_value_t = 0)]
    blue: u8,
    #[arg(long, default_value = "time-domain")]
    style: OscilloscopeStyle,
}

#[derive(Parser)]
struct BlinkCommand {
    #[arg(long, default_value_t = 100)]
    width: u32,
    #[arg(long, default_value_t = 100)]
    height: u32,
    #[arg(long, short, default_value_t = 127)]
    red: u8,
    #[arg(long, short, default_value_t = 127)]
    green: u8,
    #[arg(long, short, default_value_t = 127)]
    blue: u8,
}

#[derive(Subcommand)]
enum Command {
    Oscilloscope(OscilloscopeCommand),
    Blink(BlinkCommand),
}

impl Command {
    fn width(&self) -> u32 {
        match self {
            Self::Oscilloscope(oscilloscope_command) => {
                oscilloscope_command.width
            }
            Self::Blink(blink_command) => blink_command.width,
        }
    }
    fn height(&self) -> u32 {
        match self {
            Self::Oscilloscope(oscilloscope_command) => {
                oscilloscope_command.height
            }
            Self::Blink(blink_command) => blink_command.height,
        }
    }
}

#[derive(Parser)]
#[command(name = "caw_viz_udp_app")]
#[command(
    about = "App for launching visualization for a caw synthesizer that receives data to visualize over udp"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(long)]
    server: String,
    #[arg(long)]
    title: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
struct OscilloscopeUiState {
    style: OscilloscopeStyle,
    scale: f32,
    line_width: u32,
    alpha_scale: u8,
}

impl PersistData for OscilloscopeUiState {
    const NAME: &'static str = "oscilloscope_ui";
}

#[derive(Default)]
struct ScopeState {
    samples: VecDeque<(f32, f32)>,
}

struct App {
    server: String,
    canvas: Canvas<Window>,
    event_pump: EventPump,
    title: String,
}

impl App {
    fn handle_event_common(event: Event, title: &str) {
        match event {
            Event::Quit { .. } => std::process::exit(0),
            Event::Window {
                win_event: WindowEvent::Moved(x, y),
                ..
            } => {
                (WindowPosition { x, y }).save_(title);
            }
            _ => (),
        }
    }

    fn run_oscilloscope(
        mut self,
        args: OscilloscopeCommand,
    ) -> anyhow::Result<()> {
        self.canvas.window_mut().set_resizable(true);
        let mut viz_udp_client = oscilloscope::Client::new(self.server)?;
        let mut scope_state = ScopeState::default();
        let rgb = Rgb24::new(args.red, args.green, args.blue);
        let mut window_size = WindowSize {
            width: args.width,
            height: args.height,
        };
        let max_num_samples = args.max_num_samples;
        let mut ui_state = {
            let args = args;
            if let Some(ui_state) = OscilloscopeUiState::load_(&self.title) {
                ui_state
            } else {
                OscilloscopeUiState {
                    style: args.style,
                    scale: args.scale,
                    line_width: args.line_width,
                    alpha_scale: args.alpha_scale,
                }
            }
        };
        loop {
            for event in self.event_pump.poll_iter() {
                Self::handle_event_common(event.clone(), self.title.as_str());
                match event {
                    Event::MouseWheel { y, .. } => {
                        let ratio = 1.1;
                        if y > 0 {
                            ui_state.scale *= ratio;
                        } else if y < 0 {
                            ui_state.scale /= ratio;
                        }
                        ui_state.save_(&self.title);
                    }
                    Event::KeyDown {
                        scancode: Some(scancode),
                        ..
                    } => {
                        let (line_width_delta, alpha_scale_delta) =
                            match scancode {
                                Scancode::Up => (0, 10),
                                Scancode::Down => (0, -10),
                                Scancode::Left => (-1, 0),
                                Scancode::Right => (1, 0),
                                Scancode::Num1 => {
                                    ui_state.style =
                                        OscilloscopeStyle::TimeDomain;
                                    ui_state.save_(&self.title);
                                    continue;
                                }
                                Scancode::Num2 => {
                                    ui_state.style =
                                        OscilloscopeStyle::TimeDomainStereo;
                                    ui_state.save_(&self.title);
                                    continue;
                                }
                                Scancode::Num3 => {
                                    ui_state.style = OscilloscopeStyle::Xy;
                                    ui_state.save_(&self.title);
                                    continue;
                                }
                                _ => (0, 0),
                            };
                        ui_state.line_width = ((ui_state.line_width as i32)
                            + line_width_delta)
                            .clamp(1, 20)
                            as u32;
                        ui_state.alpha_scale = (ui_state.alpha_scale as i32
                            + alpha_scale_delta)
                            .clamp(1, 255)
                            as u8;
                        ui_state.save_(&self.title);
                    }
                    Event::Window {
                        win_event: WindowEvent::Resized(new_width, new_height),
                        ..
                    } => {
                        window_size.width = new_width as u32;
                        window_size.height = new_height as u32;
                        window_size.save_(&self.title);
                    }
                    _ => (),
                }
            }
            while let Ok(true) = viz_udp_client.recv_sample() {
                for sample_pair in viz_udp_client.pairs() {
                    scope_state.samples.push_back(sample_pair);
                }
                while scope_state.samples.len() > max_num_samples {
                    scope_state.samples.pop_front();
                }
            }
            self.canvas.set_draw_color(Color::RGB(0, 0, 0));
            self.canvas.clear();
            let screen_size = Coord {
                x: window_size.width as i32,
                y: window_size.height as i32,
            };
            match ui_state.style {
                OscilloscopeStyle::TimeDomain => {
                    let num_samples_to_draw = screen_size.x as usize;
                    let sample_mean_iter = scope_state
                        .samples
                        .iter()
                        .rev()
                        .take(num_samples_to_draw)
                        .rev()
                        .map(|(left, right)| (left + right) / 2.0);
                    self.canvas
                        .set_draw_color(Color::RGBA(rgb.r, rgb.g, rgb.b, 255));
                    let mut prev = None;
                    for (x, sample) in sample_mean_iter.enumerate() {
                        let x = x as i32;
                        let y = screen_size.y
                            - ((sample * ui_state.scale) as i32
                                + (screen_size.y / 2));
                        let coord = Coord { x, y };
                        if let Some(prev) = prev {
                            for Coord { x, y } in
                                line_2d::coords_between(prev, coord)
                            {
                                let rect = Rect::new(
                                    x,
                                    y,
                                    ui_state.line_width,
                                    ui_state.line_width,
                                );
                                let _ = self.canvas.fill_rect(rect);
                            }
                        }
                        prev = Some(coord);
                    }
                }
                OscilloscopeStyle::TimeDomainStereo => {
                    let num_samples_to_draw = screen_size.x as usize;
                    let make_sample_pair_iter = || {
                        scope_state
                            .samples
                            .iter()
                            .rev()
                            .take(num_samples_to_draw)
                            .rev()
                    };
                    let sample_left_iter =
                        make_sample_pair_iter().map(|(x, _)| x);
                    let sample_right_iter =
                        make_sample_pair_iter().map(|(_, x)| x);
                    self.canvas
                        .set_draw_color(Color::RGBA(rgb.r, rgb.g, rgb.b, 255));
                    let mut prev = None;
                    for (x, sample) in sample_left_iter.enumerate() {
                        let x = x as i32;
                        let y = screen_size.y
                            - ((sample * ui_state.scale) as i32
                                + (screen_size.y / 3));
                        let coord = Coord { x, y };
                        if let Some(prev) = prev {
                            for Coord { x, y } in
                                line_2d::coords_between(prev, coord)
                            {
                                let rect = Rect::new(
                                    x,
                                    y,
                                    ui_state.line_width,
                                    ui_state.line_width,
                                );
                                let _ = self.canvas.fill_rect(rect);
                            }
                        }
                        prev = Some(coord);
                    }
                    let mut prev = None;
                    for (x, sample) in sample_right_iter.enumerate() {
                        let x = x as i32;
                        let y = screen_size.y
                            - ((sample * ui_state.scale) as i32
                                + ((2 * screen_size.y) / 3));
                        let coord = Coord { x, y };
                        if let Some(prev) = prev {
                            for Coord { x, y } in
                                line_2d::coords_between(prev, coord)
                            {
                                let rect = Rect::new(
                                    x,
                                    y,
                                    ui_state.line_width,
                                    ui_state.line_width,
                                );
                                let _ = self.canvas.fill_rect(rect);
                            }
                        }
                        prev = Some(coord);
                    }
                }
                OscilloscopeStyle::Xy => {
                    let mut coord_iter =
                        scope_state.samples.iter().map(|(left, right)| {
                            Coord {
                                x: (left * ui_state.scale) as i32,
                                y: (right * ui_state.scale) as i32,
                            } + screen_size / 2
                        });
                    let mut prev = if let Some(first) = coord_iter.next() {
                        first
                    } else {
                        Coord::new(0, 0)
                    };
                    for (i, coord) in coord_iter.enumerate() {
                        let alpha = ((ui_state.alpha_scale as usize * i)
                            / max_num_samples)
                            .min(255) as u8;
                        self.canvas.set_draw_color(Color::RGBA(
                            rgb.r, rgb.g, rgb.b, alpha,
                        ));
                        for Coord { x, y } in
                            line_2d::coords_between(prev, coord)
                        {
                            let rect = Rect::new(
                                x,
                                y,
                                ui_state.line_width,
                                ui_state.line_width,
                            );
                            let _ = self.canvas.fill_rect(rect);
                        }
                        prev = coord;
                    }
                }
            }
            self.canvas.present();
        }
    }

    fn run_blink(
        mut self,
        args: BlinkCommand,
        title: String,
    ) -> anyhow::Result<()> {
        let mut viz_udp_client = blink::Client::new(self.server)?;
        let font = load_font(16)?;
        let text_surface = font
            .render(title.as_str())
            .blended(Color::WHITE)
            .map_err(|e| anyhow!("{e}"))?;
        let texture_creator = self.canvas.texture_creator();
        let text_texture = text_surface.as_texture(&texture_creator)?;
        let text_texture_query = text_texture.query();
        // Render the title centred at the bottom of the window.
        let text_rect = Rect::new(
            (args.width as i32 - text_texture_query.width as i32) / 2,
            args.height as i32 - text_texture_query.height as i32 - 5,
            text_texture_query.width,
            text_texture_query.height,
        );
        let mut prev_mean = 0.0;
        loop {
            for event in self.event_pump.poll_iter() {
                Self::handle_event_common(event.clone(), self.title.as_str());
            }
            let mut total = 0.0;
            let mut count = 0;
            while let Ok(true) = viz_udp_client.recv_sample() {
                for sample in viz_udp_client.iter() {
                    total += sample;
                    count += 1;
                }
            }
            self.canvas.set_draw_color(Color::RGB(0, 0, 0));
            self.canvas.clear();
            let mean = if count > 0 {
                total / (count as f32)
            } else {
                prev_mean
            };
            prev_mean = mean;
            let adjust_color = |c| ((mean * (c as f32)) as u32).min(255) as u8;
            self.canvas.set_draw_color(Color::RGB(
                adjust_color(args.red),
                adjust_color(args.green),
                adjust_color(args.blue),
            ));
            let border = 20;
            self.canvas
                .fill_rect(Rect::new(
                    border as i32,
                    border as i32 - 5,
                    args.width - (border * 2),
                    args.height - (border * 2),
                ))
                .unwrap();
            self.canvas
                .copy(&text_texture, None, Some(text_rect))
                .map_err(|e| anyhow!("{e}"))?;
            self.canvas.present();
        }
    }
}

fn main() -> anyhow::Result<()> {
    let Cli {
        command,
        server,
        title,
    } = Cli::parse();
    let sdl_context = sdl2::init().map_err(|e| anyhow!(e))?;
    let video_subsystem = sdl_context.video().map_err(|e| anyhow!(e))?;
    let (width, height) = match WindowSize::load(&title) {
        Err(_) => (command.width(), command.height()),
        Ok(WindowSize { width, height }) => (width, height),
    };
    let window_position = match WindowPosition::load(&title) {
        Err(_) => None,
        Ok(window_position) => Some(window_position),
    };
    let mut window_builder = match command {
        Command::Oscilloscope(_) => {
            video_subsystem.window(title.as_str(), width, height)
        }
        Command::Blink(_) => video_subsystem.window("", width, height),
    };
    window_builder.always_on_top();
    if let Some(WindowPosition { x, y }) = window_position {
        window_builder.position(x, y);
    }
    let window = window_builder.build()?;
    let mut canvas = window
        .into_canvas()
        .target_texture()
        .present_vsync()
        .build()?;
    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
    let event_pump = sdl_context.event_pump().map_err(|e| anyhow!(e))?;
    let app = App {
        server,
        event_pump,
        canvas,
        title: title.clone(),
    };
    match command {
        Command::Oscilloscope(mut oscilloscope_command) => {
            oscilloscope_command.width = width;
            oscilloscope_command.height = height;
            app.run_oscilloscope(oscilloscope_command)
        }
        Command::Blink(blink_command) => app.run_blink(blink_command, title),
    }
}
