pub mod builder;
pub mod clock;
pub mod envelope;
pub mod filters;
pub mod keyboard;
pub mod loopers;
pub mod patches;
pub mod sampler;
pub mod sequencers;
pub mod sf64_filter_builder_trait;
pub mod templates;
pub mod util;

mod biquad_filter;
mod freeverb;
#[cfg(feature = "midi")]
pub mod midi;
mod moog_ladder_low_pass_filter;
pub mod music;
pub mod oscillator;
pub mod signal;
pub mod signal_arithmetic;

pub mod prelude {
    #[cfg(feature = "midi")]
    pub use crate::midi::{
        MidiChannel, MidiControllerTable, MidiEvent, MidiEvents, MidiMessage, MidiMessages,
        MidiMonophonic, MidiPlayer, MidiPlayerMonophonic, MidiVoice,
    };
    pub use crate::{
        builder::{
            env::adsr_linear_01,
            filter::{
                compress, delay, delay_s, down_sample, echo, high_pass_butterworth,
                high_pass_chebyshev, low_pass_butterworth, low_pass_chebyshev,
                low_pass_moog_ladder, quantize, quantize_to_scale, reverb, sample_and_hold,
                saturate,
            },
            gate::{
                periodic_gate, periodic_gate_hz, periodic_gate_s, periodic_trigger,
                periodic_trigger_hz, periodic_trigger_s,
            },
            loopers::{clocked_midi_note_monophonic_looper, clocked_trigger_looper},
            oscillator::{oscillator, oscillator_hz, oscillator_s},
            patches::{pulse_pwm, pulse_pwm_hz, supersaw, supersaw_hz},
            sampler::sampler,
        },
        keyboard::{KeyEvent, VoiceDesc},
        music::{freq_hz_of_midi_index, note_name, semitone_ratio, AbstractChord, Note, NoteName},
        oscillator::Waveform,
        sampler::{Sample, Sampler},
        sequencers::bitwise_pattern_triggers_8,
        signal::{
            const_, first_some, freq_hz, freq_s, mean, noise, noise_01, sfreq_hz, sfreq_s,
            sfreq_to_hz, sfreq_to_s, sum, Freq, Gate, Sf64, Sfreq, Signal, Su8, Trigger,
        },
        util::{
            bitwise_trigger_router_64, generic_sample_and_hold, trigger_split_cycle,
            weighted_random_choice, with_fix,
        },
    };
}
