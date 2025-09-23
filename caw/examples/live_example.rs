use caw::prelude::*;
use std::thread;

fn main() {
    env_logger::init();
    let volume = sv_default();
    let out: LiveStereoOut = live_stereo_viz_udp(oscilloscope::Config {
        ..Default::default()
    })
    .with_volume(volume.clone());
    volume.set(knob("volume").initial_value_01(0.5).build());
    let tempo_s = knob("tempo s").build();
    let (cutoff_hz, res, lpf_space) = xy_with_space("lpf")
        .axis_label_x("cutoff_hz")
        .axis_label_y("resonance")
        .build();
    let cutoff_hz = sv(cutoff_hz);
    let res = sv(res * 2.);
    let clock = sv(periodic_trig_s(tempo_s.clone())
        .build()
        .viz_blink("clock".to_string(), Default::default()));
    let keyboard = computer_keyboard("keys").start_note(note::B_1).build_();
    let space_button = keyboard.space_button().shared();
    let key_events = keyboard.key_events().shared();
    let length = 16;
    out.set_channel(|channel| {
        let voice = key_events.clone().mono_voice();
        let (note, gate) = key_looper(voice.gated_note(), clock.clone())
            .clearing(space_button.clone())
            .length(length)
            .name("keys")
            .build()
            .ungated();
        let cutoff_hz =
            value_looper(cutoff_hz.clone(), clock.clone(), lpf_space.clone())
                .length(length)
                .build();
        let env = adsr(gate)
            .key_press_trig(clock.clone())
            .a(tempo_s.clone() * knob("attack").build())
            .r(tempo_s.clone() * knob("release").build())
            .s(knob("sustain").build())
            .d(tempo_s.clone() * knob("decay").build())
            .build()
            .exp_01(1.)
            .shared();
        let voice = (super_saw(note.freq_hz()).build() * env.clone()).filter(
            low_pass::default(10. + (env.clone() * cutoff_hz * 15000.))
                .q(res.clone()),
        );
        voice
            .viz_oscilloscope_stereo("voice", channel, Default::default())
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
