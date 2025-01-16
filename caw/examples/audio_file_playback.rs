use caw::prelude::*;
use clap::Parser;

fn sig(
    _input: Input,
    audio_file_buf: Vec<f32>,
    _channel: Channel,
) -> Sig<impl SigT<Item = f32>> {
    sample_playback(audio_file_buf).build()
}

#[derive(Parser, Debug)]
struct Args {
    wav_path: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::StereoOscillographics)
        .line_width(2)
        .stride(4)
        .fade(true)
        .build();
    let input = window.input();
    let audio_file_buf = read_wav_stereo(args.wav_path)?;
    window.play_stereo(
        Stereo::new(
            sig(input.clone(), audio_file_buf.left, Channel::Left),
            sig(input.clone(), audio_file_buf.right, Channel::Right),
        ),
        Default::default(),
    )
}
