use caw_core_next::*;
use caw_modules::*;

fn signal() -> Sig<impl SigT<Item = f32>> {
    oscillator(waveform::Saw, 100.0).build()
}

fn main() {
    let mut buffered_signal = signal();
    let ctx = SigCtx {
        sample_rate_hz: 3000.0,
        batch_index: 0,
        num_samples: 42,
    };
    let buf = buffered_signal.sample(&ctx);
    for x in buf.iter() {
        println!("{}", x);
    }
}
