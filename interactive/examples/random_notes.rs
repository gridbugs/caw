use currawong_interactive::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};

const C_MAJOR_SCALE: &[NoteName] = &[
    NoteName::A,
    NoteName::B,
    NoteName::C,
    NoteName::D,
    NoteName::E,
    NoteName::F,
    NoteName::G,
];

fn make_scale_base_freqs(note_names: &[NoteName]) -> Vec<Sfreq> {
    note_names
        .into_iter()
        .map(|&name| const_(name.in_octave(0).freq()))
        .collect()
}

fn random_note_c_major(base_hz: Sf64, range_hz: Sf64) -> Sfreq {
    sfreq_hz(base_hz + (noise_01() * range_hz))
        .filter(quantize_to_scale(make_scale_base_freqs(C_MAJOR_SCALE)).build())
}

fn super_saw_osc(freq_hz: Sf64, detune: Sf64, n: usize, gate: Gate) -> Sf64 {
    let trigger = gate.to_trigger_rising_edge();
    (0..n)
        .map(|i| {
            let delta_hz = ((i + 1) as f64 * &detune) / n as f64;
            let osc = oscillator_hz(Waveform::Saw, &freq_hz * (1 + &delta_hz))
                .reset_trigger(&trigger)
                .build()
                + oscillator_hz(Waveform::Saw, &freq_hz * (1 - &delta_hz))
                    .reset_trigger(&trigger)
                    .build();
            osc / (i as f64 + 1.0)
        })
        .sum::<Sf64>()
        + oscillator_hz(Waveform::Saw, &freq_hz).build()
}

fn voice(freq: Sfreq, gate: Gate) -> Sf64 {
    let freq_hz = freq.hz();
    let osc = super_saw_osc(freq_hz.clone(), const_(0.01), 1, gate.clone())
        + super_saw_osc(freq_hz.clone() * 2.0, const_(0.01), 1, gate.clone())
        + super_saw_osc(freq_hz.clone() * 1.25, const_(0.01), 1, gate.clone());
    let env_amp = adsr_linear_01(&gate).attack_s(0.1).release_s(8.0).build();
    let env_lpf = adsr_linear_01(&gate)
        .attack_s(0.1)
        .release_s(4.0)
        .build()
        .exp_01(1.0);
    (osc.filter(low_pass_moog_ladder(1000 * &env_lpf).resonance(2.0).build())
        + osc.filter(low_pass_moog_ladder(10000 * &env_lpf).build()))
    .mul_lazy(&env_amp)
}

fn random_replace_loop(
    trigger: Trigger,
    anchor: Sfreq,
    palette: Sfreq,
    length: usize,
    replace_probability_01: Sf64,
    anchor_probability_01: Sf64,
) -> Sfreq {
    let mut rng = StdRng::from_entropy();
    let mut sequence: Vec<Option<Freq>> = vec![None; length];
    let mut index = 0;
    let mut anchor_on_0 = false;
    let mut first_note = true;
    Signal::from_fn_mut(move |ctx| {
        let trigger = trigger.sample(ctx);
        if trigger {
            if rng.gen::<f64>() < replace_probability_01.sample(ctx) {
                sequence[index] = Some(palette.sample(ctx));
            }
            if index == 0 {
                anchor_on_0 = rng.gen::<f64>() < anchor_probability_01.sample(ctx);
            }
        }
        let freq = if first_note {
            first_note = false;
            anchor.sample(ctx)
        } else if anchor_on_0 && index == 0 {
            anchor.sample(ctx)
        } else if let Some(freq) = sequence[index] {
            freq
        } else {
            let freq = palette.sample(ctx);
            sequence[index] = Some(freq);
            freq
        };
        if trigger {
            index = (index + 1) % sequence.len();
        }
        freq
    })
}

fn synth_signal(trigger: Trigger, _input: Input) -> Sf64 {
    let freq = random_replace_loop(
        trigger.clone(),
        const_(NoteName::A.in_octave(1).freq()),
        random_note_c_major(const_(80.0), const_(200.0)),
        4,
        const_(0.1),
        const_(0.5),
    );
    let gate = trigger.to_gate_with_duration_s(0.1);
    let modulate = 1.0
        - oscillator_s(Waveform::Triangle, 60.0)
            .build()
            .signed_to_01();
    let lfo = oscillator_hz(Waveform::Sine, &modulate * 8.0).build();
    voice(freq, gate)
        .filter(down_sample(1.0 + &modulate * 10.0).build())
        .filter(low_pass_moog_ladder(10000.0 + &lfo * 2000.0).build())
        .filter(
            compress()
                .threshold(2.0)
                .scale(1.0 + &modulate * 2.0)
                .ratio(0.1)
                .build(),
        )
        .filter(high_pass_butterworth(10.0).build())
}

fn drum_signal(trigger: Trigger) -> Sf64 {
    const CYMBAL: usize = 0;
    const SNARE: usize = 1;
    const KICK: usize = 2;
    let drum_pattern = {
        let cymbal = 1 << CYMBAL;
        let snare = 1 << SNARE;
        let kick = 1 << KICK;
        vec![
            cymbal | kick,
            cymbal,
            cymbal | snare,
            cymbal,
            cymbal | kick,
            cymbal,
            cymbal | snare,
            cymbal,
            cymbal | kick,
            cymbal,
            cymbal | snare,
            cymbal,
            cymbal | kick,
            cymbal | kick,
            cymbal | snare,
            cymbal,
        ]
    };
    let drum_sequence = bitwise_pattern_triggers_8(trigger, drum_pattern).triggers;
    match &drum_sequence.as_slice() {
        &[cymbal_trigger, snare_trigger, kick_trigger, ..] => {
            cymbal(cymbal_trigger.clone())
                + snare(snare_trigger.clone())
                + kick(kick_trigger.clone())
        }
        _ => panic!(),
    }
}

fn signal(trigger: Trigger, input: Input) -> Sf64 {
    (synth_signal(trigger.divide(16), input.clone()) + drum_signal(trigger.divide(1))) * 0.2
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = signal(periodic_trigger_hz(4.0).build(), window.input());
    window.play(signal)
}

fn kick(trigger: Trigger) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.1;
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 120;
    let osc = oscillator_hz(Waveform::Triangle, freq_hz).build();
    let env_amp = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    osc.mul_lazy(&env_amp)
        .filter(compress().ratio(0.02).scale(16.0).build())
}

fn snare(trigger: Trigger) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.1;
    let noise = noise().filter(compress().ratio(0.1).scale(100.0).build());
    let env = adsr_linear_01(&clock)
        .release_s(duration_s * 1.0)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    let noise = noise
        .filter(low_pass_moog_ladder(10000.0).resonance(2.0).build())
        .filter(down_sample(10.0).build());
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 240;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz)
        .reset_trigger(trigger)
        .build();
    (noise + osc)
        .filter(down_sample(10.0).build())
        .mul_lazy(&env)
}

fn cymbal(trigger: Trigger) -> Sf64 {
    let gate = trigger.to_gate();
    let osc = noise();
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_butterworth(100.0).build());
    osc.filter(low_pass_moog_ladder(10000 * &env).build())
        .filter(high_pass_butterworth(6000.0).build())
}
