use caw::prelude::*;
use std::thread;

fn main() {
    let volume = sv_default();
    let out: LiveStereoOut = live_stereo_viz_udp(oscilloscope::Config {
        ..Default::default()
    })
    .with_volume(volume.clone());
    volume.set(knob("volume").initial_value_01(0.2).build());
    let tempo_s = sv(knob("tempo s").build() * 2.);
    let clock = sv(periodic_trig_s(tempo_s.clone()).build());
    let env_ = sv({
        let mut scope = blink::Server::new("env", Default::default()).unwrap();
        adsr(clock.clone().trig_to_gate(tempo_s.clone() / 2.))
            .a(tempo_s.clone() / 8.)
            .r(tempo_s.clone() / 2.)
            .build()
            .exp_01(1.)
            .with_buf(move |buf| {
                let _ = scope.send_samples(buf);
            })
    });
    let keys = computer_keyboard("keys")
        .start_note(note::B_1)
        .build()
        .shared();
    let space_button =
        keys.clone().controllers().get_01(0).is_positive().shared();
    let key_events = keys.clone().key_events().shared();
    out.set(|| {
        let voice = key_events.clone().mono_voice();
        let (note, gate) = sequence_looper(
            voice.gated_note(),
            clock.clone(),
            space_button.clone(),
            8,
        )
        .ungated();
        let env = adsr(gate)
            .a(tempo_s.clone() / 24.)
            .r(tempo_s.clone() / 2.)
            .s(0.75)
            .d(tempo_s.clone() / 24.)
            .build()
            .exp_01(1.);
        super_saw(note.freq_hz())
            .build()
            .filter(
                low_pass::default(10. + env_.clone() * 0. + (env * 15000.))
                    .q(0.2),
            )
            .filter(
                chorus()
                    .lfo_rate_hz(0.1)
                    .delay_s(0.001)
                    .depth_s(0.002)
                    .mix_01(0.5)
                    .feedback_ratio(0.5),
            )
            .filter(reverb::default().room_size(0.9).damping(0.1))

        //            .filter(reverb())
        //            .filter(chorus())
    });
    thread::park();
}
