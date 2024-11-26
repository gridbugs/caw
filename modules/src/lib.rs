pub mod oscillator;
pub use oscillator::{
    oscillator,
    waveform::{self, *},
};

pub mod envelope;
pub use envelope::adsr_linear_01;

pub mod super_saw;
pub use super_saw::super_saw;

pub mod sample_and_hold;
pub use sample_and_hold::sample_and_hold;

pub mod low_pass_moog_ladder;
pub use low_pass_moog_ladder::low_pass_moog_ladder;

pub mod low_pass_butterworth;
pub use low_pass_butterworth::low_pass_butterworth;

pub mod low_pass_chebyshev;
pub use low_pass_chebyshev::low_pass_chebyshev;

pub mod low_pass {
    pub use super::low_pass_butterworth as butterworth;
    pub use super::low_pass_chebyshev as chebyshev;
    pub use super::low_pass_moog_ladder as moog_ladder;

    pub use moog_ladder as default;
}

pub mod high_pass_butterworth;
pub use high_pass_butterworth::high_pass_butterworth;

pub mod high_pass_chebyshev;
pub use high_pass_chebyshev::high_pass_chebyshev;

pub mod high_pass {
    pub use super::high_pass_butterworth as butterworth;
    pub use super::high_pass_chebyshev as chebyshev;

    pub use butterworth as default;
}

pub mod band_pass_butterworth;
pub use band_pass_butterworth::band_pass_butterworth;

pub mod band_pass_chebyshev;
pub use band_pass_chebyshev::band_pass_chebyshev;

pub mod band_pass {
    pub use super::band_pass_butterworth as butterworth;
    pub use super::band_pass_chebyshev as chebyshev;
    pub mod centered {
        pub use crate::band_pass_butterworth::band_pass_butterworth_centered as butterworth;
        pub use crate::band_pass_chebyshev::band_pass_chebyshev_centered as chebyshev;

        pub use butterworth as default;
    }

    pub use butterworth as default;
}

pub mod reverb_freeverb;
pub use reverb_freeverb::reverb_freeverb;

pub mod reverb {
    pub use super::reverb_freeverb as freeverb;

    pub use freeverb as default;
}

mod low_level;
