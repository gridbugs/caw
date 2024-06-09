use caw_core_next::{Signal, Waveform};
use caw_proc_macros::signal_builder;

signal_builder!(
    #[allow(unused)]
    struct OscillatorBuilder<T> {
        #[signal]
        freq: f64,
        #[signal]
        #[default = Waveform::Sine]
        waveform: Waveform,
        #[signal]
        #[default = 0.0]
        reset_offset_01: f64,
        other: T,
        #[default = 1]
        other2: usize,
        other3: String,
    }
);
