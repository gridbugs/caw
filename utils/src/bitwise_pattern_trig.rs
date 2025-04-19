use caw_core::{Sig, SigT};

/// Returns an array of 8 trigger signals, each corresponding to one of the bits in the patterns.
/// On each trigger pulse rising edge the current pattern is advanced and those of the returned
/// triggers corresponding to 1s in the current pattern will receive pulses.
pub fn bitwise_pattern_trigs_8(
    trig: impl SigT<Item = bool>,
    patterns: Vec<u8>,
) -> [Sig<impl SigT<Item = bool>>; 8] {
    let trig = Sig(trig).gate_to_trig_rising_edge();
    let mut i = 0;
    // This will be `Some(<pattern>)` on samples where the trigger is high (and also advance the
    // current pattern) and `None` otherwise.
    let pattern_sig = Sig(trig)
        .on(move || {
            let pattern = patterns[i];
            i = (i + 1) % patterns.len();
            pattern
        })
        .shared();
    let vec: Vec<_> = (0..8)
        .map(move |i| {
            pattern_sig.clone().map(move |pattern_opt| {
                pattern_opt.is_some_and(|pattern| pattern & (1 << i) != 0)
            })
        })
        .collect();
    vec.try_into().unwrap_or_else(|_| unreachable!())
}
