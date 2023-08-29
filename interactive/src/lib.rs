pub struct Window {
    pub title: String,
    pub width_px: u32,
    pub height_px: u32,
}

impl Window {
    pub fn run(&self) -> Result<(), String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let window = video_subsystem
            .window(self.title.as_str(), self.width_px, self.height_px)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;
        let mut event_pump = sdl_context.event_pump()?;
        'running: loop {
            for event in event_pump.poll_iter() {
                use sdl2::event::Event;
                match event {
                    Event::Quit { .. } => break 'running,
                    _ => println!("{:?}", event),
                }
            }
        }
        Ok(())
    }
}
