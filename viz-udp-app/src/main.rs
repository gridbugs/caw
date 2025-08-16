use anyhow::anyhow;
use caw_viz_udp_app_lib::VizUdpClient;
use clap::Parser;
use line_2d::Coord;
use sdl2::{event::Event, pixels::Color, rect::Rect};
use std::collections::VecDeque;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    server: String,
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
}

#[derive(Default)]
struct ScopeState {
    samples: VecDeque<(f32, f32)>,
}

fn main() -> anyhow::Result<()> {
    let mut args = Args::parse();
    let sdl_context = sdl2::init().map_err(|e| anyhow!(e))?;
    let video_subsystem = sdl_context.video().map_err(|e| anyhow!(e))?;
    let window = video_subsystem
        .window("CAW Visualization", args.width, args.height)
        .always_on_top()
        .build()?;
    let mut canvas = window
        .into_canvas()
        .target_texture()
        .present_vsync()
        .build()?;
    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
    let mut event_pump = sdl_context.event_pump().map_err(|e| anyhow!(e))?;
    let mut viz_udp_client = VizUdpClient::new(args.server)?;
    let mut scope_state = ScopeState::default();
    loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => std::process::exit(0),
                Event::MouseWheel { y, .. } => {
                    let ratio = 1.1;
                    if y > 0 {
                        args.scale *= ratio;
                    } else {
                        args.scale /= ratio;
                    }
                }
                Event::MouseMotion {
                    mousestate, xrel, ..
                } => {
                    if mousestate.left() {
                        args.line_width = ((args.line_width as i32) + xrel)
                            .clamp(1, 20)
                            as u32;
                    }
                    if mousestate.right() {
                        args.alpha_scale = (args.alpha_scale as i32 + xrel)
                            .clamp(1, 255)
                            as u8;
                    }
                }
                _ => (),
            }
        }
        while let Ok(true) = viz_udp_client.recv_sample() {
            for sample_pair in viz_udp_client.pairs() {
                scope_state.samples.push_back(sample_pair);
            }
            while scope_state.samples.len() > args.max_num_samples {
                scope_state.samples.pop_front();
            }
        }
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        let screen_size = Coord {
            x: args.width as i32,
            y: args.height as i32,
        };
        let mut coord_iter = scope_state.samples.iter().map(|(left, right)| {
            Coord {
                x: (left * args.scale) as i32,
                y: (right * args.scale) as i32,
            } + screen_size / 2
        });
        let mut prev = if let Some(first) = coord_iter.next() {
            first
        } else {
            Coord::new(0, 0)
        };
        for (i, coord) in coord_iter.enumerate() {
            let alpha = ((args.alpha_scale as usize * i) / args.max_num_samples)
                .min(255) as u8;
            canvas.set_draw_color(Color::RGBA(
                args.red, args.green, args.blue, alpha,
            ));
            for Coord { x, y } in line_2d::coords_between(prev, coord) {
                let rect = Rect::new(x, y, args.line_width, args.line_width);
                let _ = canvas.fill_rect(rect);
            }
            prev = coord;
        }
        canvas.present();
    }
}
