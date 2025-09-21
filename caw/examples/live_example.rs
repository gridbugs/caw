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
    let tempo_s = sv(knob("tempo s").build() * 0.5);
    let (cutoff_hz, res) = xy("lpf")
        .axis_label_x("cutoff_hz")
        .axis_label_y("resonance")
        .build();
    let cutoff_hz = sv(cutoff_hz);
    let res = sv(res * 2.);
    let clock = sv(periodic_trig_s(tempo_s.clone())
        .build()
        .viz_blink("clock".to_string(), Default::default()));
    let ck = computer_keyboard("keys").start_note(note::B_0).build_();
    let space_button = ck.space_button().shared();
    let key_events = ck.key_events().shared();
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
        let voice = (super_saw(note.freq_hz()).build() * env.clone())
            .filter(
                low_pass::default(
                    10. + (env.clone() * cutoff_hz.clone() * 15000.),
                )
                .q(res.clone()),
            )
            .viz_oscilloscope_stereo("voice", channel, Default::default());
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
