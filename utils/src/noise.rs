use caw_core::{Sig, SigT};
use rand::{Rng, SeedableRng, rngs::StdRng};

pub fn white() -> Sig<impl SigT<Item = f32>> {
    let mut rng = StdRng::from_os_rng();
    Sig::from_fn(move |_| rng.random::<f32>() * 2. - 1.)
}

/// Approximation of brown noise made by integrating white noise with a leaky integrator and
/// hand-tuning.
pub fn brown() -> Sig<impl SigT<Item = f32>> {
    let mut total = 0.;
    white().map_mut(move |x| {
        total = (total * 0.99) + (x * 0.1);
        total
    })
}

/// Based on some of the code snippets from https://www.firstpr.com.au/dsp/pink-noise/
pub fn pink() -> Sig<impl SigT<Item = f32>> {
    let mut b = [0.; 7];
    white().map_mut(move |x| {
        b[0] = 0.99886 * b[0] + x * 0.0555179;
        b[1] = 0.99332 * b[1] + x * 0.0750759;
        b[2] = 0.96900 * b[2] + x * 0.153852;
        b[3] = 0.86650 * b[3] + x * 0.3104856;
        b[4] = 0.55000 * b[4] + x * 0.5329522;
        b[5] = -0.7616 * b[5] - x * 0.0168980;
        let pink = b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + x * 0.5362;
        b[6] = x * 0.115926;
        pink * 0.25
    })
}
