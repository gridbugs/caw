use anyhow::anyhow;
use sdl2::pixels::Color;
use std::{
    thread,
    time::{Duration, Instant},
};

const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / 60);

pub struct Window {
    pub title: String,
    pub width_px: u32,
    pub height_px: u32,
}

impl Window {
    pub fn run<F: FnMut()>(&self, mut f: F) -> anyhow::Result<()> {
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
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();
        // Skip this many frames to prevent choppy audio on startup.
        let mut warmup_frames = 15;
        let mut event_pump = sdl_context.event_pump().map_err(|e| anyhow!(e))?;
        'running: loop {
            let frame_start = Instant::now();
            for event in event_pump.poll_iter() {
                use sdl2::event::Event;
                match event {
                    Event::Quit { .. } => break 'running,
                    _ => println!("{:?}", event),
                }
            }
            if warmup_frames > 0 {
                warmup_frames -= 1;
            } else {
                f();
            }
            let since_frame_start = frame_start.elapsed();
            if let Some(until_next_frame) = FRAME_DURATION.checked_sub(since_frame_start) {
                thread::sleep(until_next_frame);
            }
        }
        Ok(())
    }
}
