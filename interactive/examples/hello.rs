use ibis_interactive::{sample_player::OutputStream, window::Window};

fn main() -> anyhow::Result<()> {
    let window = Window {
        title: "hello".to_string(),
        width_px: 960,
        height_px: 720,
    };
    let stream: OutputStream<f32> = OutputStream::new()?;
    println!("Sample Rate Hz: {}", stream.sample_rate_hz());
    window.run(|| {})
}
