pub mod builder;
pub mod clock;
pub mod envelope;
pub mod filters;
pub mod loopers;
pub mod sampler;

#[cfg(feature = "midi")]
pub mod midi;
pub mod music;
pub mod oscillator;
pub mod signal;
pub mod signal_arithmetic;

pub mod prelude {
    #[cfg(feature = "midi")]
    pub use crate::midi::{
        MidiChannel, MidiControllerTable, MidiPlayer, MidiPlayerMonophonic, MidiVoice,
    };
    pub use crate::{
        builder::{
            filter::{
                echo, high_pass_butterworth, high_pass_chebyshev, low_pass_butterworth,
                low_pass_chebyshev, low_pass_moog_ladder, sample_and_hold, saturate,
            },
            gate::{
                periodic_gate, periodic_gate_hz, periodic_gate_s, periodic_trigger,
                periodic_trigger_hz, periodic_trigger_s,
            },
            loopers::{clocked_midi_note_monophonic_looper, clocked_trigger_looper},
            sampler::sampler,
            signal::{adsr_linear_01, oscillator, oscillator_hz, oscillator_s},
        },
        music::{freq_hz_of_midi_index, semitone_ratio, Note, NoteName},
        oscillator::Waveform,
        sampler::{Sample, Sampler},
        signal::{
            const_, mean, noise, noise_01, sfreq_hz, sfreq_s, sfreq_to_hz, sfreq_to_s, sum, Freq,
            Gate, Sf64, Sfreq, Signal, Su8, Trigger,
        },
    };
}
