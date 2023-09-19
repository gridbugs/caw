pub mod builder;
pub mod clock;
pub mod envelope;
pub mod filters;
pub mod midi;
pub mod music;
pub mod oscillator;
pub mod signal;
pub mod signal_arithmetic;

pub mod prelude {
    pub use crate::{
        builder::{
            filter::{
                high_pass_butterworth, high_pass_chebyshev, low_pass_butterworth,
                low_pass_chebyshev, low_pass_moog_ladder, saturate,
            },
            gate::{periodic_gate, periodic_gate_hz, periodic_gate_s},
            signal::{adsr_linear_01, oscillator, oscillator_hz, oscillator_s},
        },
        midi::{MidiControllerTable, MidiPlayer, MidiVoice},
        oscillator::Waveform,
        signal::{
            const_, mean, noise, sfreq_hz, sfreq_s, sfreq_to_hz, sfreq_to_s, sum, Freq, Gate, Sf64,
            Sfreq, Signal, Trigger,
        },
    };
}
