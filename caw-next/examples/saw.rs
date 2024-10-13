use caw_builders::*;
use caw_core_next::*;

fn signal() -> impl Signal<Item = f64> {
    oscillator(waveform::Saw, freq_hz(100.0)).build()
}

fn main() {
    todo!()
}
