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
    let env = sv({
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
    let key_events = computer_keyboard("keys").build().key_events().shared();
    out.set(|| {
        let voice = key_events.clone().mono_voice();
        super_saw(voice.note.freq_hz())
            .build()
            .filter(low_pass::default(100. + env.clone() * 4000.).q(0.5))
            .filter(reverb())
    });
    thread::park();
}
