use caw::prelude::*;
use std::thread;

fn main() {
    env_logger::init();
    let volume = cell_default();
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
    let cutoff_hz = cell(cutoff_hz);
    let res = cell(res * 2.);
    let clock = cell(
        periodic_trig_s(tempo_s.clone())
            .build()
            .viz_blink("clock".to_string(), Default::default()),
    );
    let keyboard = computer_keyboard("keys").start_note(note::B_1).build_();
    let space_button = keyboard.space_button().shared();
    let key_events = keyboard.key_events().shared();
    let (drum_bits, drum_space) = num_keys_bits_7_with_space("drums").build();
    let length = 32;
    let drum_bits_loop = value_looper(drum_bits, clock.clone(), drum_space)
        .persist_with_name("drums")
        .length(length)
        .build()
        .shared();
    let drums = (vec![
        clock
            .clone()
            .and(drum_bits_loop.clone().map(|x| x & (1 << 0) != 0))
            .trig(drum::kick())
            .boxed(),
        clock
            .clone()
            .and(drum_bits_loop.clone().map(|x| x & (1 << 1) != 0))
            .trig(drum::snare())
            .boxed(),
        clock
            .clone()
            .and(drum_bits_loop.clone().map(|x| x & (1 << 2) != 0))
            .trig(drum::hat_closed())
            .boxed(),
    ]
    .into_iter()
    .sum::<Sig<_>>()
        * knob("drum vol").build())
    .shared();
    out.set_channel(|channel| {
        let voice = key_events.clone().mono_voice();
        let (note, gate) = key_looper(voice.gated_note(), clock.clone())
            .clearing(space_button.clone())
            .length(length)
            .persist_with_name("keys")
            .build()
            .ungated();
        let vib_hz = knob("vibrato").build() * 20.;
        let max = semitone_ratio_sig(knob("vib scale").build() * 4.).shared();
        let min = (1.0 / max.clone()).shared();
        let vib = sine(vib_hz).build().signed_to_range(min, max);
        let cutoff_hz =
            value_looper(cutoff_hz.clone(), clock.clone(), lpf_space.clone())
                .persist_with_name("low_pass")
                .length(length)
                .build();
        let env = adsr(
            (gate & clock.clone())
                .trig_to_gate(tempo_s.clone() * knob("duration").build()),
        )
        .key_press_trig(clock.clone())
        .a(tempo_s.clone() * knob("attack").build() * 4.)
        .r(tempo_s.clone() * knob("release").build() * 4.)
        .s(knob("sustain").build())
        .d(tempo_s.clone() * knob("decay").build() * 4.)
        .build()
        .exp_01(1.)
        .shared();
        let (glide_enable, glide_enable_record) =
            button_with_space("glide enable").build();
        let note_freq = (note.freq_hz() * vib)
            .filter_enable(
                value_looper(glide_enable, clock.clone(), glide_enable_record)
                    .persist_with_name("glide")
                    .length(length)
                    .build(),
                low_pass::butterworth(10. * knob("glide").build()),
            )
            .shared();
        let lfo = sine(knob("lfo rate").build() * note_freq.clone())
            .build()
            .signed_to_01()
            * knob("lfo amp").build()
            * 2_000.;
        let voice = (saw(note_freq)
            .reset_offset_01(channel.circle_phase_offset_01())
            .build()
            * env.clone())
        .filter(
            low_pass::default(
                lfo + (env.clone()
                    * cutoff_hz
                    * 15000.
                    * knob("cutoff_scale").build()),
            )
            .q(res.clone()),
        )
        .filter_enable(
            switch("band pass").build(),
            band_pass::centered::default(
                knob("mid").build() * 1_000.,
                knob("width").build() * 4.,
            ),
        )
        .filter(
            compressor()
                .ratio(0.05)
                .threshold(1.0)
                .scale(knob("distort").build() * 10.),
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
            .filter(high_pass::default(1.))
            + drums.clone()
    });
    thread::park();
}
