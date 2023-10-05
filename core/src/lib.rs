pub mod builder;
pub mod clock;
pub mod envelope;
pub mod filters;

#[cfg(feature = "midi")]
pub mod midi;
pub mod music;
pub mod oscillator;
pub mod signal;
pub mod signal_arithmetic;

pub mod prelude {
    #[cfg(feature = "midi")]
    pub use crate::midi::{MidiControllerTable, MidiPlayer, MidiPlayerMonophonic, MidiVoice};
    pub use crate::{
        builder::{
            filter::{
                echo, high_pass_butterworth, high_pass_chebyshev, low_pass_butterworth,
                low_pass_chebyshev, low_pass_moog_ladder, sample_and_hold, saturate,
            },
            gate::{periodic_gate, periodic_gate_hz, periodic_gate_s},
            signal::{adsr_linear_01, oscillator, oscillator_hz, oscillator_s},
        },
        music::{freq_hz_of_midi_index, semitone_ratio, Note, NoteName},
        oscillator::Waveform,
        signal::{
            const_, mean, noise, sfreq_hz, sfreq_s, sfreq_to_hz, sfreq_to_s, sum, Freq, Gate, Sf64,
            Sfreq, Signal, Trigger,
        },
    };
}
