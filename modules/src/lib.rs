mod low_level;

pub mod oscillator;
pub use oscillator::{
    oscillator,
    waveform::{self, *},
};

pub mod envelope_generator;
pub use envelope_generator::adsr_linear_01;

pub mod envelope_follower;
pub use envelope_follower::envelope_follower;

pub mod super_saw;
pub use super_saw::super_saw;

pub mod sample_and_hold;
pub use sample_and_hold::sample_and_hold;

pub mod low_pass_moog_ladder;
pub use low_pass_moog_ladder::{
    low_pass_moog_ladder_huovilainen, low_pass_moog_ladder_oberheim,
};

pub mod low_pass_butterworth;
pub use low_pass_butterworth::low_pass_butterworth;

pub mod low_pass_chebyshev;
pub use low_pass_chebyshev::low_pass_chebyshev;

pub mod low_pass {
    pub use super::low_pass_butterworth as butterworth;
    pub use super::low_pass_chebyshev as chebyshev;
    pub mod moog_ladder {
        pub use super::super::low_pass_moog_ladder_huovilainen as huovilainen;
        pub use super::super::low_pass_moog_ladder_oberheim as oberheim;
        pub use oberheim as default;
    }

    pub use moog_ladder::default;
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

pub mod sample_playback;
pub use sample_playback::sample_playback;

pub mod periodic_trig;
pub use periodic_trig::{periodic_trig_hz, periodic_trig_s};

pub mod periodic_gate;
pub use periodic_gate::periodic_gate_s;

pub mod delay_periodic;
pub use delay_periodic::delay_periodic_s;

pub mod delay_triggered;
pub use delay_triggered::delay_triggered;

pub mod delay {
    pub use super::delay_periodic::delay_periodic_s as periodic_s;
    pub use super::delay_triggered::delay_triggered as triggered;
}
