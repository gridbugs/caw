use crate::signal::{sfreq_hz, Sf64, Sfreq, Signal};

pub fn detune<F: FnMut(Sfreq) -> Sf64>(
    resolution: usize,
    freq: Sfreq,
    ratio_delta: Sf64,
    mut make_oscillator: F,
) -> Sf64 {
    if resolution == 0 {
        make_oscillator(freq)
    } else {
        let scaled_ratio_delta = ratio_delta / (resolution as f64);
        let signals = [make_oscillator(freq.clone())]
            .into_iter()
            .chain(
                (0..resolution)
                    .map(move |i| {
                        [
                            make_oscillator(sfreq_hz(
                                freq.hz()
                                    * (1.0
                                        + (&scaled_ratio_delta
                                            * ((i + 1) as f64))),
                            )),
                            make_oscillator(sfreq_hz(
                                freq.hz()
                                    * (1.0
                                        - (&scaled_ratio_delta
                                            * ((i + 1) as f64))),
                            )),
                        ]
                    })
                    .flatten(),
            )
            .collect::<Vec<_>>();
        Signal::from_fn(move |ctx| {
            let mut out = 0.0;
            for signal in &signals {
                out += signal.sample(ctx);
            }
            out / signals.len() as f64
        })
    }
}
