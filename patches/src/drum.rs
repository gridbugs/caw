use caw_core_next::{
    frame_sig_shared, FrameSig, FrameSigT, Sig, SigT, Triggerable,
};
use caw_modules::*;

// A sweep which ends at `pitch_base` hz and starts `pitch_scale_octaves` number of octaves higher
// than `pitch_base`. The sweep is linear in frequency over a pediod of `
struct PitchSweep<H, P, B, S>
where
    H: SigT<Item = f32>,
    P: SigT<Item = f32>,
    B: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    hold_s: H,
    period_s: P,
    pitch_scale_octaves: B,
    pitch_base_hz: S,
}

impl<H, P, B, S> Triggerable for PitchSweep<H, P, B, S>
where
    H: SigT<Item = f32>,
    P: SigT<Item = f32>,
    B: SigT<Item = f32>,
    S: SigT<Item = f32>,
{
    type Item = f32;

    fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
    where
        T: FrameSigT<Item = bool>,
    {
        let trig = FrameSig(trig).gate_to_trig_rising_edge();
        let trig = frame_sig_shared(trig);
        let gate = FrameSig(trig.clone()).into_sig().trig_to_gate(self.hold_s);
        let freq_hz_env = adsr_linear_01(gate)
            .key_press_trig(trig.clone())
            .release_s(self.period_s)
            .build()
            .exp_01(2.);
        let freq_hz = Sig(self.pitch_base_hz)
            * (1. + (Sig(self.pitch_scale_octaves) * freq_hz_env));
        oscillator(Sine, freq_hz).build()
    }
}

struct AmpEnv<H, R>
where
    H: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    hold_s: H,
    release_s: R,
}

impl<H, R> Triggerable for AmpEnv<H, R>
where
    H: SigT<Item = f32>,
    R: SigT<Item = f32>,
{
    type Item = f32;

    fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
    where
        T: FrameSigT<Item = bool>,
    {
        let trig = FrameSig(trig).gate_to_trig_rising_edge();
        let trig = frame_sig_shared(trig);
        let gate = FrameSig(trig.clone()).into_sig().trig_to_gate(self.hold_s);
        adsr_linear_01(gate)
            .key_press_trig(trig)
            .attack_s(0.001)
            .release_s(self.release_s)
            .build()
            .exp_01(2.)
    }
}

struct NoiseFilterSweep<R, S, E>
where
    R: SigT<Item = f32>,
    S: SigT<Item = f32>,
    E: SigT<Item = f32>,
{
    release_s: R,
    base_cutoff_hz: S,
    start_cutoff_offset_hz: E,
}

impl<R, S, E> Triggerable for NoiseFilterSweep<R, S, E>
where
    R: SigT<Item = f32>,
    S: SigT<Item = f32>,
    E: SigT<Item = f32>,
{
    type Item = f32;

    fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
    where
        T: FrameSigT<Item = bool>,
    {
        let trig = FrameSig(trig).gate_to_trig_rising_edge();
        let env = adsr_linear_01(trig)
            .release_s(self.release_s)
            .build()
            .exp_01(2.)
            .shared();
        noise::white().filter(low_pass::default(
            Sig(self.base_cutoff_hz)
                + (Sig(self.start_cutoff_offset_hz) * env.clone()),
        )) * env.clone()
    }
}

mod kick {
    use caw_builder_proc_macros::builder;
    use caw_core_next::{
        frame_sig_shared, sig_shared, FrameSigT, Sig, SigT, Triggerable,
    };
    use caw_modules::*;

    use super::{AmpEnv, NoiseFilterSweep, PitchSweep};

    struct Props<P, N>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
    {
        period_s: P,
        noise_amp: N,
    }

    impl<P, N> Props<P, N>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
    {
        fn new(period_s: P, noise_amp: N) -> Self {
            Self {
                period_s,
                noise_amp,
            }
        }
    }

    builder! {
        #[constructor = "kick"]
        #[constructor_doc = "Kick drum"]
        #[build_fn = "Props::new"]
        #[build_ty = "Props<P, N>"]
        #[generic_setter_type_name = "X"]
        pub struct PropsBuilder {
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "P"]
            #[default = 0.05]
            period_s: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "N"]
            #[default = 1.0]
            noise_amp: f32,

        }
    }

    impl<P, N> Triggerable for PropsBuilder<P, N>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
    {
        type Item = f32;

        fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
        where
            T: FrameSigT<Item = bool>,
        {
            sig(self.build(), trig)
        }
    }

    fn sig<D, N>(
        props: Props<D, N>,
        trig: impl FrameSigT<Item = bool>,
    ) -> impl SigT<Item = f32>
    where
        D: SigT<Item = f32>,
        N: SigT<Item = f32>,
    {
        let period_s = sig_shared(props.period_s);
        let trig = frame_sig_shared(trig);
        let pitch_sweep = trig.clone().trig(PitchSweep {
            hold_s: 0.01,
            period_s: period_s.clone(),
            pitch_base_hz: 50.,
            pitch_scale_octaves: 4.,
        });
        let noise = trig.clone().trig(NoiseFilterSweep {
            release_s: period_s.clone(),
            base_cutoff_hz: 10_000.,
            start_cutoff_offset_hz: 10_000.,
        });
        let amp_env = trig.clone().trig(AmpEnv {
            hold_s: 0.1,
            release_s: period_s.clone(),
        });
        let sig = (pitch_sweep + (noise * Sig(props.noise_amp))).shared();
        (sig.clone() + (sig.clone().filter(low_pass::default(500.)) * 3.))
            * amp_env
            * 0.5
    }
}

mod snare {
    use caw_builder_proc_macros::builder;
    use caw_core_next::{
        frame_sig_shared, sig_shared, FrameSigT, Sig, SigT, Triggerable,
    };
    use caw_modules::*;

    use super::{AmpEnv, NoiseFilterSweep, PitchSweep};

    struct Props<P, N>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
    {
        period_s: P,
        noise_amp: N,
    }

    impl<P, N> Props<P, N>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
    {
        fn new(period_s: P, noise_amp: N) -> Self {
            Self {
                period_s,
                noise_amp,
            }
        }
    }

    builder! {
        #[constructor = "snare"]
        #[constructor_doc = "Snare drum"]
        #[build_fn = "Props::new"]
        #[build_ty = "Props<P, N>"]
        #[generic_setter_type_name = "X"]
        pub struct PropsBuilder {
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "P"]
            #[default = 0.05]
            period_s: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "N"]
            #[default = 1.0]
            noise_amp: f32,

        }
    }

    impl<P, N> Triggerable for PropsBuilder<P, N>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
    {
        type Item = f32;

        fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
        where
            T: FrameSigT<Item = bool>,
        {
            sig(self.build(), trig)
        }
    }

    fn sig<D, N>(
        props: Props<D, N>,
        trig: impl FrameSigT<Item = bool>,
    ) -> impl SigT<Item = f32>
    where
        D: SigT<Item = f32>,
        N: SigT<Item = f32>,
    {
        let period_s = sig_shared(props.period_s);
        let trig = frame_sig_shared(trig);
        let pitch_sweep = trig.clone().trig(PitchSweep {
            hold_s: 0.01,
            period_s: period_s.clone(),
            pitch_base_hz: 50.,
            pitch_scale_octaves: 4.,
        });
        let noise = trig.clone().trig(NoiseFilterSweep {
            release_s: period_s.clone() * 2.0,
            base_cutoff_hz: 10_000.,
            start_cutoff_offset_hz: 10_000.,
        });
        let amp_env = trig.clone().trig(AmpEnv {
            hold_s: 0.1,
            release_s: period_s.clone(),
        });
        let sig = pitch_sweep + (noise * Sig(props.noise_amp));
        sig.filter(high_pass::default(400.0)) * amp_env
    }
}

mod hat_closed {
    use caw_builder_proc_macros::builder;
    use caw_core_next::{FrameSig, FrameSigT, SigT, Triggerable};

    use super::NoiseFilterSweep;

    struct Props<P>
    where
        P: SigT<Item = f32>,
    {
        period_s: P,
    }

    impl<P> Props<P>
    where
        P: SigT<Item = f32>,
    {
        fn new(period_s: P) -> Self {
            Self { period_s }
        }
    }

    builder! {
        #[constructor = "hat_closed"]
        #[constructor_doc = "Closed hi-hat"]
        #[build_fn = "Props::new"]
        #[build_ty = "Props<P>"]
        #[generic_setter_type_name = "X"]
        pub struct PropsBuilder {
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "P"]
            #[default = 0.05]
            period_s: f32,

        }
    }

    impl<P> Triggerable for PropsBuilder<P>
    where
        P: SigT<Item = f32>,
    {
        type Item = f32;

        fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
        where
            T: FrameSigT<Item = bool>,
        {
            sig(self.build(), trig)
        }
    }

    fn sig<D>(
        props: Props<D>,
        trig: impl FrameSigT<Item = bool>,
    ) -> impl SigT<Item = f32>
    where
        D: SigT<Item = f32>,
    {
        FrameSig(trig).trig(NoiseFilterSweep {
            release_s: props.period_s,
            base_cutoff_hz: 20_000.,
            start_cutoff_offset_hz: -16_000.,
        })
    }
}

pub use hat_closed::hat_closed;
pub use kick::kick;
pub use snare::snare;
