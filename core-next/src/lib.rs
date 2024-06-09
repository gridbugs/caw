pub trait Signal {
    type Item;
}

pub enum Waveform {
    Sine,
}

impl Signal for Waveform {
    type Item = Self;
}

impl Signal for f64 {
    type Item = Self;
}
