use caw_builders::*;
use caw_core_next::*;

fn signal() -> impl Signal<Item = f64> {
    oscillator(waveform::Saw, freq_hz(100.0)).build()
}

fn main() {
    let mut buffered_signal = signal().buffered();
    let ctx = SignalCtx {
        sample_rate_hz: 3000.0,
        batch_index: 0,
    };
    buffered_signal.sample_batch(&ctx, 42);
    for x in buffered_signal.samples() {
        println!("{}", x);
    }
}
