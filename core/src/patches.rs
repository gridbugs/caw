use crate::{
    builder,
    oscillator::Waveform,
    signal::{const_, freq_hz, noise, Sf64, Sfreq, Trigger},
    templates,
};

pub fn supersaw(
    resolution: usize,
    freq: Sfreq,
    ratio: Sf64,
    reset_trigger: Trigger,
    reset_offset_01: Sf64,
) -> Sf64 {
    templates::detune(resolution, freq, ratio, |freq| {
        builder::oscillator::oscillator(Waveform::Saw, freq)
            .reset_trigger(&reset_trigger)
            .reset_offset_01(&reset_offset_01)
            .build()
    })
}

pub fn pulse_pwm(
    osc_freq: Sfreq,
    pwm_freq: Sfreq,
    offset_01: Sf64,
    scale_01: Sf64,
    reset_trigger: Trigger,
    reset_offset_01: Sf64,
) -> Sf64 {
    let offset = offset_01 * 0.5;
    let scale = scale_01 * (0.5 - &offset);
    let lfo = builder::oscillator::oscillator(Waveform::Triangle, pwm_freq)
        .reset_trigger(reset_trigger)
        .reset_offset_01(reset_offset_01)
        .build()
        .signed_to_01()
        * scale
        + offset;
    builder::oscillator::oscillator(Waveform::Pulse, osc_freq)
        .pulse_width_01(lfo)
        .build()
}

pub fn kick_drum(trigger: Trigger) -> Sf64 {
    let pitch_hold_s = const_(0.01);
    let pitch_gate = trigger.to_gate_with_duration_s(pitch_hold_s);
    let pitch_release_s = const_(0.05);
    let pitch_env = builder::env::adsr_linear_01(pitch_gate)
        .key_press(&trigger)
        .release_s(pitch_release_s)
        .build()
        .exp_01(2.0);
    let pitch_base = freq_hz(50.0);
    let pitch_scale_octaves = const_(4.0);
    let osc = builder::oscillator::oscillator_hz(
        Waveform::Sine,
        pitch_base.hz() * (1.0 + (pitch_scale_octaves * pitch_env)),
    )
    .build();
    let noise_hold_s = const_(0.01);
    let noise_gate = trigger.to_gate_with_duration_s(noise_hold_s);
    let noise_release_s = const_(0.04);
    let noise_env = builder::env::adsr_linear_01(noise_gate)
        .key_press(&trigger)
        .release_s(noise_release_s)
        .build()
        .exp_01(2.0);
    let noise = noise()
        .filter(builder::filter::low_pass_moog_ladder(10000.0 + &noise_env * 10000).build())
        * noise_env
        * 2.0;
    let filtered_osc = (osc + noise)
        .filter(builder::filter::low_pass_moog_ladder(4000.0).build())
        .mix(|dry| 3.0 * dry.filter(builder::filter::low_pass_moog_ladder(500.0).build()));
    let amp_hold_s = const_(0.1);
    let amp_gate = trigger.to_gate_with_duration_s(amp_hold_s);
    let amp_release_s = const_(0.1);
    let amp_env = builder::env::adsr_linear_01(amp_gate)
        .key_press(&trigger)
        .attack_s(0.001)
        .release_s(amp_release_s)
        .build()
        .exp_01(2.0);
    filtered_osc * amp_env
}

pub fn snare_drum(trigger: Trigger) -> Sf64 {
    let pitch_hold_s = const_(0.01);
    let pitch_gate = trigger.to_gate_with_duration_s(pitch_hold_s);
    let pitch_release_s = const_(0.05);
    let pitch_env = builder::env::adsr_linear_01(pitch_gate)
        .key_press(&trigger)
        .release_s(pitch_release_s)
        .build()
        .exp_01(2.0);
    let pitch_base = freq_hz(50.0);
    let pitch_scale_octaves = const_(4.0);
    let osc = builder::oscillator::oscillator_hz(
        Waveform::Sine,
        pitch_base.hz() * (1.0 + (pitch_scale_octaves * pitch_env)),
    )
    .build();
    let noise_hold_s = const_(0.01);
    let noise_gate = trigger.to_gate_with_duration_s(noise_hold_s);
    let noise_release_s = const_(0.1);
    let noise_env = builder::env::adsr_linear_01(noise_gate)
        .key_press(&trigger)
        .release_s(noise_release_s)
        .build()
        .exp_01(2.0);
    let noise = noise()
        .filter(builder::filter::low_pass_moog_ladder(10000.0 + &noise_env * 10000).build())
        * noise_env
        * 0.5;
    let filtered_osc = (osc + noise).filter(builder::filter::high_pass_butterworth(200.0).build());
    let amp_hold_s = const_(0.1);
    let amp_gate = trigger.to_gate_with_duration_s(amp_hold_s);
    let amp_release_s = const_(0.1);
    let amp_env = builder::env::adsr_linear_01(amp_gate)
        .key_press(&trigger)
        .attack_s(0.001)
        .release_s(amp_release_s)
        .build()
        .exp_01(2.0);
    let reverb_hold_s = const_(0.01);
    let reverb_gate = trigger.to_gate_with_duration_s(reverb_hold_s);
    let reverb_release_s = const_(0.2);
    let reverb_env = builder::env::adsr_linear_01(reverb_gate)
        .release_s(reverb_release_s)
        .build();
    (filtered_osc * amp_env).mix(|dry| {
        0.5 * reverb_env
            * dry.filter(
                builder::filter::reverb()
                    .room_size(0.2)
                    .damping(0.1)
                    .build(),
            )
    })
}

pub fn closed_hat(trigger: Trigger) -> Sf64 {
    let noise_hold_s = const_(0.01);
    let noise_gate = trigger.to_gate_with_duration_s(noise_hold_s);
    let noise_release_s = const_(0.05);
    let noise_env = builder::env::adsr_linear_01(noise_gate)
        .key_press(&trigger)
        .release_s(noise_release_s)
        .build()
        .exp_01(1.0);
    let noise = noise()
        .filter(builder::filter::low_pass_moog_ladder(20000 - &noise_env * 10000).build())
        * noise_env;
    let reverb_hold_s = const_(0.01);
    let reverb_gate = trigger.to_gate_with_duration_s(reverb_hold_s);
    let reverb_release_s = const_(0.2);
    let reverb_env = builder::env::adsr_linear_01(reverb_gate)
        .release_s(reverb_release_s)
        .build();
    noise.mix(|dry| {
        1.0 * reverb_env
            * dry.filter(
                builder::filter::reverb()
                    .room_size(0.2)
                    .damping(0.1)
                    .build(),
            )
    }) * 0.5
}
