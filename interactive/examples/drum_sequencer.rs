use caw_core::builder::filter::envelope_follower;
use caw_interactive::prelude::*;

fn kick(trigger: &Trigger) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.1;
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 120;
    let osc = oscillator_hz(Waveform::Triangle, freq_hz)
        .reset_trigger(trigger)
        .build();
    let env_amp = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    osc.mul_lazy(&env_amp)
        .filter(compress().ratio(0.02).scale(16.0).build())
}

fn snare(trigger: &Trigger) -> Sf64 {
    let clock = trigger.to_gate();
    let duration_s = 0.1;
    let noise = noise();
    let detune = 0.0002;
    let noise = (&noise
        + &noise.filter(delay_s(&detune))
        + &noise.filter(delay_s(&detune * 2.0))
        + &noise.filter(delay_s(&detune * 3.0)))
        .filter(compress().ratio(0.1).scale(100.0).build());
    let env = adsr_linear_01(&clock)
        .release_s(duration_s * 1.0)
        .build()
        .exp_01(1.0)
        .filter(low_pass_moog_ladder(1000.0).build());
    let noise =
        noise.filter(low_pass_moog_ladder(10000.0).resonance(2.0).build());
    let freq_hz = adsr_linear_01(&clock)
        .release_s(duration_s)
        .build()
        .exp_01(1.0)
        * 240;
    let osc = oscillator_hz(Waveform::Pulse, freq_hz)
        .reset_trigger(trigger)
        .build();
    (noise + osc).mul_lazy(&env)
}

fn cymbal(trigger: &Trigger) -> Sf64 {
    let gate = trigger.to_gate();
    let env = adsr_linear_01(gate)
        .release_s(0.1)
        .build()
        .filter(low_pass_moog_ladder(1000.0).build());
    noise()
        .filter(low_pass_moog_ladder(10000 * &env).build())
        .filter(high_pass_butterworth(6000.0).build())
}

const CYMBAL: usize = 0;
const SNARE: usize = 1;
const KICK: usize = 2;

fn drum_loop(trigger: Trigger, pattern: Vec<u8>) -> (Sf64, Trigger) {
    let drum_sequence = bitwise_pattern_triggers_8(trigger, pattern);
    (
        match &drum_sequence.triggers.as_slice() {
            &[cymbal_trigger, snare_trigger, kick_trigger, ..] => {
                cymbal(cymbal_trigger)
                    + snare(snare_trigger)
                    + kick(kick_trigger)
            }
            _ => panic!(),
        },
        drum_sequence.complete,
    )
}

fn basic_drum_loop(trigger: &Trigger) -> Sf64 {
    kick(&trigger.divide(8))
}

fn voice(trigger: Trigger, input: Input) -> Sf64 {
    let cymbal = 1 << CYMBAL;
    let snare = 1 << SNARE;
    let kick = 1 << KICK;
    #[rustfmt::skip]
    let drum_patterns = [
        vec![
            cymbal | snare,
            cymbal,
            cymbal,
        ],
        vec![
            cymbal | snare,
            cymbal,
            cymbal,
        ],
        vec![
            cymbal,
        ],
        vec![
            kick,
        ],
    ];
    let drum_signal = with_fix({
        let trigger = trigger.clone();
        |loop_selection: Signal<u64>| {
            let (drum_signals, completion_triggers): (Vec<Sf64>, Vec<Trigger>) =
                bitwise_trigger_router_64(trigger, loop_selection)
                    .into_iter()
                    .zip(drum_patterns.into_iter())
                    .map(|(trigger, drum_pattern)| {
                        drum_loop(trigger, drum_pattern)
                    })
                    .unzip();
            let drum_signal = sum(drum_signals);
            let completion_trigger = Trigger::any(completion_triggers);
            #[rustfmt::skip]
            let pattern_index_weights = vec![
                (1 << 0, const_(4.0)),
                (1 << 1, const_(1.0)),
                (1 << 2, const_(1.0)),
                (1 << 3, const_(0.5)),
            ];
            let loop_selection = generic_sample_and_hold(
                weighted_random_choice(pattern_index_weights),
                completion_trigger,
            );
            (loop_selection, drum_signal)
        }
    });
    let voice = (basic_drum_loop(&trigger) + drum_signal)
        .filter(low_pass_moog_ladder(20000.0).build())
        .filter(compress().scale(0.8).build());
    let ef = voice.filter(envelope_follower().build());
    voice
        + (oscillator_hz(Waveform::Pulse, 400.0 * input.mouse.x_01())
            .pulse_width_01(input.mouse.y_01())
            .build()
            * ef)
}

fn main() -> anyhow::Result<()> {
    let window = Window::builder()
        .stable(true)
        .spread(1)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let signal = voice(periodic_trigger_hz(8.0).build(), window.input());
    window.play(signal)
}
