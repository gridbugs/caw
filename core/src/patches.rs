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
    hard_sync: Sf64,
) -> Sf64 {
    templates::detune(resolution, freq, ratio, |freq| {
        builder::oscillator::oscillator(Waveform::Saw, freq)
            .reset_trigger(&reset_trigger)
            .reset_offset_01(&reset_offset_01)
            .hard_sync(&hard_sync)
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

pub mod drum {
    use crate::{
        builder,
        oscillator::Waveform,
        signal::{const_, freq_hz, noise, Sf64, Trigger},
    };

    fn sine_wave_pitch_sweep(trigger: &Trigger) -> Sf64 {
        let pitch_hold_s = const_(0.01);
        let pitch_gate = trigger.to_gate_with_duration_s(pitch_hold_s);
        let pitch_release_s = const_(0.05);
        let pitch_env = builder::env::adsr_linear_01(pitch_gate)
            .key_press(trigger)
            .release_s(pitch_release_s)
            .build()
            .exp_01(2.0);
        let pitch_base = freq_hz(50.0);
        let pitch_scale_octaves = const_(4.0);
        builder::oscillator::oscillator_hz(
            Waveform::Sine,
            pitch_base.hz() * (1.0 + (pitch_scale_octaves * pitch_env)),
        )
        .build()
    }

    fn noise_lpf_sweep(
        trigger: &Trigger,
        release_s: f64,
        sweep_start_hz: f64,
        sweep_end_offset_hz: f64,
    ) -> Sf64 {
        let hold_s = const_(0.01);
        let gate = trigger.to_gate_with_duration_s(hold_s);
        let env = builder::env::adsr_linear_01(gate)
            .key_press(trigger)
            .release_s(release_s)
            .build()
            .exp_01(2.0);
        noise()
            .filter(
                builder::filter::low_pass_moog_ladder(
                    sweep_start_hz + (&env * sweep_end_offset_hz),
                )
                .build(),
            )
            .mul_lazy(&env)
    }

    fn amp_env(trigger: &Trigger) -> Sf64 {
        let hold_s = const_(0.1);
        let gate = trigger.to_gate_with_duration_s(hold_s);
        let release_s = const_(0.1);
        builder::env::adsr_linear_01(gate)
            .key_press(trigger)
            .attack_s(0.001)
            .release_s(release_s)
            .build()
            .exp_01(2.0)
    }

    pub fn kick(trigger: Trigger, noise_level: Sf64) -> Sf64 {
        let osc = sine_wave_pitch_sweep(&trigger);
        let noise =
            noise_lpf_sweep(&trigger, 0.04, 10_000.0, 10_000.0) * noise_level;
        let filtered_osc = (osc + noise)
            .filter(builder::filter::low_pass_moog_ladder(4000.0).build())
            .mix(|dry| {
                3.0 * dry.filter(
                    builder::filter::low_pass_moog_ladder(500.0).build(),
                )
            });
        filtered_osc.mul_lazy(&amp_env(&trigger))
    }

    pub fn snare(trigger: Trigger, noise_level: Sf64) -> Sf64 {
        let osc = sine_wave_pitch_sweep(&trigger);
        let noise =
            noise_lpf_sweep(&trigger, 0.1, 10_000.0, 10_000.0) * noise_level;
        let filtered_osc = (osc + noise)
            .filter(builder::filter::high_pass_butterworth(200.0).build());
        filtered_osc.mul_lazy(&amp_env(&trigger))
    }

    pub fn hat_closed(trigger: Trigger) -> Sf64 {
        noise_lpf_sweep(&trigger, 0.05, 20_000.0, -10_000.0)
    }
}
