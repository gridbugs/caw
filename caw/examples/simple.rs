use caw::prelude::*;

fn main() {
    // Open a window that can play and visualize an audio signal.
    let window = Window::builder().build();

    // Describe the audio signal.
    let input = window.input();
    let MonoVoice {
        note,
        key_down_gate,
        ..
    } = input.keyboard.opinionated_key_events(Note::B2).mono_voice();
    let env = adsr_linear_01(key_down_gate).attack_s(0.1).build();
    let sig = oscillator(Saw, note.freq_hz())
        .build()
        .filter(
            low_pass::default(env * input.mouse.x_01() * 20_000.0)
                .resonance(input.mouse.y_01()),
        )
        .filter(chorus())
        .filter(reverb::default());

    // Play the audio signal, visualizing its waveform in the window.
    window.play_mono(sig, Default::default()).unwrap();
}
