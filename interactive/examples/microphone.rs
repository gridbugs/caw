use caw_interactive::prelude::*;

fn voice() -> Sf64 {
    microphone()
        .unwrap()
        .mix(|dry| dry.filter(reverb().room_size(0.9).build()))
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder().sane_default().build();
    window.play(voice())
}
