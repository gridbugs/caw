use caw_core_next::*;
use caw_builders::*;

fn signal() -> impl Signal<Item=f64>{
    oscillator(waveform::Saw, freq_hz(100.0)).build()
}

fn main() {
    todo!()
}
