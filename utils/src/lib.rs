use caw_core::{FrameSig, FrameSigT};

pub mod noise;

/// Returns an array of 8 trigger signals, each corresponding to one of the bits in the patterns.
/// On each trigger pulse the current pattern is advanced and those of the returned triggers
/// corresponding to 1s in the current pattern will receive pulses.
pub fn bitwise_pattern_trigs_8(
    trig: impl FrameSigT<Item = bool>,
    patterns: Vec<u8>,
) -> [FrameSig<impl FrameSigT<Item = bool>>; 8] {
    let mut i = 0;
    // This will be `Some(<pattern>)` on frames where the trigger is high (and also advance the
    // current pattern) and `None` otherwise.
    let pattern_frame_sig = FrameSig(trig)
        .on(move || {
            let pattern = patterns[i];
            i = (i + 1) % patterns.len();
            pattern
        })
        .shared();
    let vec: Vec<_> = (0..8)
        .map(move |i| {
            pattern_frame_sig.clone().map(move |pattern_opt| {
                pattern_opt
                    .map_or(false, move |pattern| pattern & (1 << i) != 0)
            })
        })
        .collect();
    vec.try_into().unwrap_or_else(|_| unreachable!())
}
