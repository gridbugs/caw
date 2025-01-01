use caw_core_next::{
    frame_sig_shared, FrameSig, FrameSigT, Sig, SigT, Triggerable,
};
use caw_modules::*;

// A sweep which ends at `pitch_base` hz and starts `pitch_start_scale_octaves` number of octaves higher
// than `pitch_base`. The sweep is linear in frequency over a pediod of `
struct PitchSweep<H, P, B, S, C>
where
    H: SigT<Item = f32>,
    P: SigT<Item = f32>,
    B: SigT<Item = f32>,
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
{
    hold_s: H,
    period_s: P,
    pitch_start_scale_octaves: B,
    pitch_base_hz: S,
    curve: C,
}

impl<H, P, B, S, C> Triggerable for PitchSweep<H, P, B, S, C>
where
    H: SigT<Item = f32>,
    P: SigT<Item = f32>,
    B: SigT<Item = f32>,
    S: SigT<Item = f32>,
    C: SigT<Item = f32>,
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
            .exp_01(self.curve);
        let freq_hz = Sig(self.pitch_base_hz)
            * (1. + (Sig(self.pitch_start_scale_octaves) * freq_hz_env));
        oscillator(Sine, freq_hz).build()
    }
}

/// A simple envelope which goes to 1 for `hold_s` seconds after a trigger then linearly goes to 0
/// over a period of `release_s` seconds.
struct AmpEnv<H, R, C>
where
    H: SigT<Item = f32>,
    R: SigT<Item = f32>,
    C: SigT<Item = f32>,
{
    hold_s: H,
    release_s: R,
    curve: C,
}

impl<H, R, C> Triggerable for AmpEnv<H, R, C>
where
    H: SigT<Item = f32>,
    R: SigT<Item = f32>,
    C: SigT<Item = f32>,
{
    type Item = f32;

    fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
    where
        T: FrameSigT<Item = bool>,
    {
        let trig = FrameSig(trig).gate_to_trig_rising_edge().shared();
        let gate = FrameSig(trig.clone()).into_sig().trig_to_gate(self.hold_s);
        adsr_linear_01(gate)
            .key_press_trig(trig)
            .release_s(self.release_s)
            .build()
            .exp_01(self.curve)
    }
}

// White noise through a low-pass filter
struct NoiseFilterSweep<R, S, E, C>
where
    R: SigT<Item = f32>,
    S: SigT<Item = f32>,
    E: SigT<Item = f32>,
    C: SigT<Item = f32>,
{
    release_s: R,
    base_cutoff_hz: S,
    start_cutoff_offset_hz: E,
    curve: C,
}

impl<R, S, E, C> Triggerable for NoiseFilterSweep<R, S, E, C>
where
    R: SigT<Item = f32>,
    S: SigT<Item = f32>,
    E: SigT<Item = f32>,
    C: SigT<Item = f32>,
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
            .exp_01(self.curve);
        noise::white().filter(low_pass::default(
            Sig(self.base_cutoff_hz) + (Sig(self.start_cutoff_offset_hz) * env),
        ))
    }
}

// A kick drum has 2 parts:
// - a sine wave with a frequency sweep
// - white noise with a low-pass filter frequency sweep
mod kick {
    use caw_builder_proc_macros::builder;
    use caw_core_next::{
        frame_sig_shared, sig_shared, FrameSigT, Sig, SigT, Triggerable,
    };
    use caw_modules::*;

    use super::{AmpEnv, NoiseFilterSweep, PitchSweep};

    struct Props<P, N, BC, BA, PB, PS, NFB, NFS, C>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
        BC: SigT<Item = f32>,
        BA: SigT<Item = f32>,
        PB: SigT<Item = f32>,
        PS: SigT<Item = f32>,
        NFB: SigT<Item = f32>,
        NFS: SigT<Item = f32>,
        C: SigT<Item = f32>,
    {
        period_s: P,
        noise_amp: N,
        bass_boost_cutoff_hz: BC,
        bass_boost_amp: BA,
        pitch_base_hz: PB,
        pitch_start_scale_octaves: PS,
        noise_filter_base_cutoff_hz: NFB,
        noise_filter_start_offset_cutoff_hz: NFS,
        curve: C,
    }

    impl<P, N, BC, BA, PB, PS, NFB, NFS, C> Props<P, N, BC, BA, PB, PS, NFB, NFS, C>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
        BC: SigT<Item = f32>,
        BA: SigT<Item = f32>,
        PB: SigT<Item = f32>,
        PS: SigT<Item = f32>,
        NFB: SigT<Item = f32>,
        NFS: SigT<Item = f32>,
        C: SigT<Item = f32>,
    {
        #[allow(clippy::too_many_arguments)]
        fn new(
            period_s: P,
            noise_amp: N,
            bass_boost_cutoff_hz: BC,
            bass_boost_amp: BA,
            pitch_base_hz: PB,
            pitch_start_scale_octaves: PS,
            noise_filter_base_cutoff_hz: NFB,
            noise_filter_start_offset_cutoff_hz: NFS,
            curve: C,
        ) -> Self {
            Self {
                period_s,
                noise_amp,
                bass_boost_cutoff_hz,
                bass_boost_amp,
                pitch_base_hz,
                pitch_start_scale_octaves,
                noise_filter_base_cutoff_hz,
                noise_filter_start_offset_cutoff_hz,
                curve,
            }
        }
    }

    builder! {
        #[constructor = "kick"]
        #[constructor_doc = "Kick drum"]
        #[build_fn = "Props::new"]
        #[build_ty = "Props<P, N, BC, BA, PB, PS, NFB, NFS, C>"]
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
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "BC"]
            #[default = 500.0]
            bass_boost_cutoff_hz: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "BA"]
            #[default = 1.0]
            bass_boost_amp: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "PB"]
            #[default = 20.]
            pitch_base_hz: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "PS"]
            #[default = 10.]
            pitch_start_scale_octaves: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "NFB"]
            #[default = 5_000.]
            noise_filter_base_cutoff_hz: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "NFS"]
            #[default = 15_000.]
            noise_filter_start_offset_cutoff_hz: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "C"]
            #[default = 1.0]
            curve: f32,
        }
    }

    impl<P, N, BC, BA, PB, PS, NFB, NFS, C> Triggerable
        for PropsBuilder<P, N, BC, BA, PB, PS, NFB, NFS, C>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
        BC: SigT<Item = f32>,
        BA: SigT<Item = f32>,
        PB: SigT<Item = f32>,
        PS: SigT<Item = f32>,
        NFB: SigT<Item = f32>,
        NFS: SigT<Item = f32>,
        C: SigT<Item = f32>,
    {
        type Item = f32;

        fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
        where
            T: FrameSigT<Item = bool>,
        {
            let props = self.build();
            let period_s = sig_shared(props.period_s);
            let curve = sig_shared(props.curve);
            let trig = frame_sig_shared(trig);
            let pitch_sweep = trig.clone().trig(PitchSweep {
                hold_s: 0.01,
                period_s: period_s.clone(),
                pitch_base_hz: props.pitch_base_hz,
                pitch_start_scale_octaves: props.pitch_start_scale_octaves,
                curve: curve.clone(),
            });
            let noise = trig.clone().trig(NoiseFilterSweep {
                release_s: period_s.clone(),
                base_cutoff_hz: props.noise_filter_base_cutoff_hz,
                start_cutoff_offset_hz: props
                    .noise_filter_start_offset_cutoff_hz,
                curve: curve.clone(),
            });
            let amp_env = trig.clone().trig(AmpEnv {
                hold_s: 0.1,
                release_s: period_s.clone(),
                curve: curve.clone(),
            });
            let sig = (pitch_sweep + (noise * Sig(props.noise_amp))).shared();
            (sig.clone()
                + (sig
                    .clone()
                    .filter(low_pass::default(props.bass_boost_cutoff_hz))
                    * Sig(props.bass_boost_amp)))
                * amp_env
        }
    }
}

mod snare {
    use caw_builder_proc_macros::builder;
    use caw_core_next::{
        frame_sig_shared, sig_shared, FrameSigT, Sig, SigT, Triggerable,
    };
    use caw_modules::*;

    use super::{AmpEnv, NoiseFilterSweep, PitchSweep};

    struct Props<P, N, PB, PS, NFB, NFS, HC, C>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
        PB: SigT<Item = f32>,
        PS: SigT<Item = f32>,
        NFB: SigT<Item = f32>,
        NFS: SigT<Item = f32>,
        HC: SigT<Item = f32>,
        C: SigT<Item = f32>,
    {
        period_s: P,
        noise_amp: N,
        pitch_base_hz: PB,
        pitch_start_scale_octaves: PS,
        noise_filter_base_cutoff_hz: NFB,
        noise_filter_start_offset_cutoff_hz: NFS,
        high_pass_filter_cutoff_hz: HC,
        curve: C,
    }

    impl<P, N, PB, PS, NFB, NFS, HC, C> Props<P, N, PB, PS, NFB, NFS, HC, C>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
        PB: SigT<Item = f32>,
        PS: SigT<Item = f32>,
        NFB: SigT<Item = f32>,
        NFS: SigT<Item = f32>,
        HC: SigT<Item = f32>,
        C: SigT<Item = f32>,
    {
        #[allow(clippy::too_many_arguments)]
        fn new(
            period_s: P,
            noise_amp: N,
            pitch_base_hz: PB,
            pitch_start_scale_octaves: PS,
            noise_filter_base_cutoff_hz: NFB,
            noise_filter_start_offset_cutoff_hz: NFS,
            high_pass_filter_cutoff_hz: HC,
            curve: C,
        ) -> Self {
            Self {
                period_s,
                noise_amp,
                pitch_base_hz,
                pitch_start_scale_octaves,
                noise_filter_base_cutoff_hz,
                noise_filter_start_offset_cutoff_hz,
                high_pass_filter_cutoff_hz,
                curve,
            }
        }
    }

    builder! {
        #[constructor = "snare"]
        #[constructor_doc = "Snare drum"]
        #[build_fn = "Props::new"]
        #[build_ty = "Props<P, N, PB, PS, NFB, NFS, HC, C>"]
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
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "PB"]
            #[default = 20.]
            pitch_base_hz: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "PS"]
            #[default = 10.]
            pitch_start_scale_octaves: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "NFB"]
            #[default = 10_000.]
            noise_filter_base_cutoff_hz: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "NFS"]
            #[default = 10_000.]
            noise_filter_start_offset_cutoff_hz: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "HC"]
            #[default = 200.0]
            high_pass_filter_cutoff_hz: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "C"]
            #[default = 1.0]
            curve: f32,
        }
    }

    impl<P, N, PB, PS, NFB, NFS, HC, C> Triggerable
        for PropsBuilder<P, N, PB, PS, NFB, NFS, HC, C>
    where
        P: SigT<Item = f32>,
        N: SigT<Item = f32>,
        PB: SigT<Item = f32>,
        PS: SigT<Item = f32>,
        NFB: SigT<Item = f32>,
        NFS: SigT<Item = f32>,
        HC: SigT<Item = f32>,
        C: SigT<Item = f32>,
    {
        type Item = f32;

        fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
        where
            T: FrameSigT<Item = bool>,
        {
            let props = self.build();
            let period_s = sig_shared(props.period_s);
            let curve = sig_shared(props.curve);
            let trig = frame_sig_shared(trig);
            let pitch_sweep = trig.clone().trig(PitchSweep {
                hold_s: 0.01,
                period_s: period_s.clone(),
                pitch_base_hz: props.pitch_base_hz,
                pitch_start_scale_octaves: props.pitch_start_scale_octaves,
                curve: curve.clone(),
            });
            let noise = trig.clone().trig(NoiseFilterSweep {
                release_s: period_s.clone(),
                base_cutoff_hz: props.noise_filter_base_cutoff_hz,
                start_cutoff_offset_hz: props
                    .noise_filter_start_offset_cutoff_hz,
                curve: curve.clone(),
            });
            let amp_env = trig.clone().trig(AmpEnv {
                hold_s: 0.1,
                release_s: period_s.clone(),
                curve: curve.clone(),
            });
            let sig = pitch_sweep + (noise * Sig(props.noise_amp));
            sig.filter(high_pass::default(props.high_pass_filter_cutoff_hz))
                * amp_env
        }
    }
}

mod hat_closed {
    use caw_builder_proc_macros::builder;
    use caw_core_next::{sig_shared, FrameSig, FrameSigT, SigT, Triggerable};
    use caw_modules::adsr_linear_01;

    use super::NoiseFilterSweep;

    struct Props<P, NFB, NFS, C>
    where
        P: SigT<Item = f32>,
        NFB: SigT<Item = f32>,
        NFS: SigT<Item = f32>,
        C: SigT<Item = f32>,
    {
        period_s: P,
        noise_filter_base_cutoff_hz: NFB,
        noise_filter_start_offset_cutoff_hz: NFS,
        curve: C,
    }

    impl<P, C, NFB, NFS> Props<P, NFB, NFS, C>
    where
        P: SigT<Item = f32>,
        NFB: SigT<Item = f32>,
        NFS: SigT<Item = f32>,
        C: SigT<Item = f32>,
    {
        fn new(
            period_s: P,
            noise_filter_base_cutoff_hz: NFB,
            noise_filter_start_offset_cutoff_hz: NFS,
            curve: C,
        ) -> Self {
            Self {
                period_s,
                noise_filter_base_cutoff_hz,
                noise_filter_start_offset_cutoff_hz,
                curve,
            }
        }
    }

    builder! {
        #[constructor = "hat_closed"]
        #[constructor_doc = "Closed hi-hat"]
        #[build_fn = "Props::new"]
        #[build_ty = "Props<P, NFB, NFS, C>"]
        #[generic_setter_type_name = "X"]
        pub struct PropsBuilder {
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "P"]
            #[default = 0.05]
            period_s: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "NFB"]
            #[default = 20_000.]
            noise_filter_base_cutoff_hz: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "NFS"]
            #[default = -10_000.]
            noise_filter_start_offset_cutoff_hz: f32,
            #[generic_with_constraint = "SigT<Item = f32>"]
            #[generic_name = "C"]
            #[default = 1.0]
            curve: f32,
        }
    }

    impl<P, NFB, NFS, C> Triggerable for PropsBuilder<P, NFB, NFS, C>
    where
        P: SigT<Item = f32>,
        NFB: SigT<Item = f32>,
        NFS: SigT<Item = f32>,
        C: SigT<Item = f32>,
    {
        type Item = f32;

        fn into_sig<T>(self, trig: T) -> impl SigT<Item = Self::Item>
        where
            T: FrameSigT<Item = bool>,
        {
            let props = self.build();
            let period_s = sig_shared(props.period_s);
            let curve = sig_shared(props.curve);
            let trig = FrameSig(trig).gate_to_trig_rising_edge().shared();
            let env = adsr_linear_01(trig.clone())
                .release_s(period_s.clone())
                .build()
                .exp_01(curve.clone());
            FrameSig(trig.clone()).trig(NoiseFilterSweep {
                release_s: period_s.clone(),
                base_cutoff_hz: props.noise_filter_base_cutoff_hz,
                start_cutoff_offset_hz: props
                    .noise_filter_start_offset_cutoff_hz,
                curve,
            }) * env
                * 2.0 // scale it so the default loudness roughly matches the other drums
        }
    }
}

pub use hat_closed::hat_closed;
pub use kick::kick;
pub use snare::snare;
