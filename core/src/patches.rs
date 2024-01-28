use crate::{
    builder,
    oscillator::Waveform,
    signal::{Sf64, Sfreq, Trigger},
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
