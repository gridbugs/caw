use std::net::{Ipv4Addr, UdpSocket};

use anyhow::anyhow;
use clap::Parser;
use sdl2::{event::Event, pixels::Color};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    server: String,
    #[arg(long, default_value_t = 640)]
    width: u32,
    #[arg(long, default_value_t = 480)]
    height: u32,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let sdl_context = sdl2::init().map_err(|e| anyhow!(e))?;
    let video_subsystem = sdl_context.video().map_err(|e| anyhow!(e))?;
    let window = video_subsystem
        .window("CAW Visualization", args.width, args.height)
        .build()?;
    let mut canvas = window
        .into_canvas()
        .target_texture()
        .present_vsync()
        .build()?;
    let mut event_pump = sdl_context.event_pump().map_err(|e| anyhow!(e))?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.connect(args.server)?;
    // TODO: send a magic number here
    assert_eq!(socket.send(&[42])?, 1);
    socket.set_nonblocking(true)?;
    loop {
        for event in event_pump.poll_iter() {
            if let Event::Quit { .. } = event {
                std::process::exit(0);
            }
        }
        // TODO: larger buffer
        let mut buf = [0];
        if let Ok(size) = socket.recv(&mut buf) {
            // TODO: check for "would block" errors here
            assert_eq!(size, 1);
            println!("{}", buf[0]);
        }
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();
    }
}
