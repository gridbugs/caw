pub mod builder;
pub mod clock;
pub mod envelope;
pub mod filters;
pub mod keyboard;
pub mod loopers;
pub mod patches;
pub mod sampler;
pub mod sequencers;
pub mod templates;
pub mod util;

mod biquad_band_pass_filter;
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
    pub use crate::midi::{MidiControllerTable, MidiEvent, MidiEvents, MidiMessage, MidiMessages};
    pub use crate::{
        builder::{
            env::adsr_linear_01,
            filter::{
                band_pass_butterworth, band_pass_butterworth_centered, compress, delay, delay_s,
                down_sample, echo, high_pass_butterworth, high_pass_chebyshev,
                low_pass_butterworth, low_pass_chebyshev, low_pass_moog_ladder, quantize,
                quantize_to_scale, reverb, sample_and_hold, saturate,
            },
            gate::{
                periodic_gate, periodic_gate_hz, periodic_gate_s, periodic_trigger,
                periodic_trigger_hz, periodic_trigger_s,
            },
            loopers::{clocked_midi_note_monophonic_looper, clocked_trigger_looper},
            oscillator::{oscillator, oscillator_hz, oscillator_s},
            patches::{
                hat_closed, kick, pulse_pwm, pulse_pwm_hz, snare, supersaw, supersaw_hz,
                triggerable,
            },
            sampler::sampler,
        },
        keyboard::{ArpeggiatorConfig, ArpeggiatorShape, ChordVoiceConfig, KeyEvent, VoiceDesc},
        music::{
            chord::{
                chord, Chord, ChordPosition, ChordType, Inversion, DIMINISHED, MAJOR, MINOR, OPEN,
                SUS_2, SUS_4,
            },
            freq_hz_of_midi_index, note, note_name, octave,
            octave::*,
            semitone_ratio, Note, NoteName, Octave,
        },
        oscillator::Waveform,
        sampler::{Sample, Sampler},
        sequencers::{bitwise_pattern_triggers_8, drum_loop_8},
        signal::{
            const_, first_some, freq_hz, freq_s, mean, noise, noise_01, sfreq_hz, sfreq_s,
            sfreq_to_hz, sfreq_to_s, sum, triggerable, Freq, Gate, Sf64, Sfreq, Signal, Su8,
            Trigger, Triggerable,
        },
        util::{
            bitwise_trigger_router_64, generic_sample_and_hold, trigger_split_cycle,
            weighted_random_choice, with_fix,
        },
    };
}
