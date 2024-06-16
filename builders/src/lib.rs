use caw_core_next::{Signal, Waveform};
use caw_proc_macros::signal_builder;

fn oscillator<T, R>(
    _freq: impl Signal<Item = f64>,
    _waveform: impl Signal<Item = Waveform>,
    _reset_offset_01: impl Signal<Item = f64>,
    _other: T,
    _other2: usize,
    _other3: String,
    reset_01: R,
) -> impl Signal<Item = f64> {
    0.0
}

signal_builder!(
    #[allow(unused)]
    #[build_fn = "oscillator"]
    #[build_ty = "impl Signal<Item = f64>"]
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
        #[default = 42]
        #[generic_with_constraint = "Into<u64>"]
        #[generic_with_constraint = "std::fmt::Display"]
        reset_01: u64,
    }
);
