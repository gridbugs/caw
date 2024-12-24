use caw_core_next::*;
use caw_interactive_next::{Input, Key, Visualization, Window};
use caw_modules::*;
use caw_patches::drum;

fn sig(input: Input) -> Sig<impl SigT<Item = f32>> {
    (input.key(Key::C).trig(drum::kick())
        + input.key(Key::X).trig(drum::snare())
        + input.key(Key::Z).trig(drum::hat_closed()))
    .filter(reverb::default().room_size(0.1).damping(0.9))
    .filter(high_pass::default(1.))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .sane_default()
        .visualization(Visualization::Oscilloscope)
        .line_width(2)
        .stride(4)
        .build();
    let input = window.input();
    window
        .play_stereo(Stereo::new_fn(|| sig(input.clone())), Default::default())
}
