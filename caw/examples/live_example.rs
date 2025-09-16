use caw::prelude::*;
use std::thread;

fn main() {
    let volume = sv_default();
    let out: LiveStereoOut = live_stereo_viz_udp(oscilloscope::Config {
        ..Default::default()
    })
    .with_volume(volume.clone());
    volume.set(knob("volume").initial_value_01(0.5).build());
    let tempo_s = sv(knob("tempo s").build() * 0.5);
    let lpf = sv(knob("lpf").build() * 0.5);
    let clock = sv(periodic_trig_s(tempo_s.clone())
        .build()
        .viz_blink("clock".to_string(), Default::default()));
    let keys = computer_keyboard("keys")
        .start_note(note::B_1)
        .build()
        .shared();
    let space_button =
        keys.clone().controllers().get_01(0).is_positive().shared();
    let key_events = keys.clone().key_events().shared();
    out.set_channel(|channel| {
        let voice = key_events.clone().mono_voice();
        let (note, gate) = key_looper(
            voice.gated_note(),
            clock.clone(),
            space_button.clone(),
            8,
        )
        .ungated();
        let env = adsr(gate)
            .key_press_trig(clock.clone())
            .a(tempo_s.clone() / 24.)
            .r(tempo_s.clone() / 2.)
            .s(0.25)
            .d(tempo_s.clone() / 2.)
            .build()
            .exp_01(1.)
            .shared();
        let voice = (super_saw(note.freq_hz()).build() * env.clone()).filter(
            low_pass::default(10. + (env.clone() * lpf.clone() * 15000.))
                .q(0.2),
        );
        voice
            .filter(
                chorus()
                    .lfo_rate_hz(0.1)
                    .delay_s(0.001)
                    .depth_s(0.002)
                    .mix_01(0.5)
                    .feedback_ratio(0.5),
            )
            .filter(reverb::default().room_size(0.9).damping(0.1))
    });
    thread::park();
}
