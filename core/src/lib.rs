mod arith;
mod sig;
pub mod sig_ops;
pub use sig::{
    sig_boxed, sig_boxed_var_const, sig_boxed_var_default,
    sig_option_first_some, sig_shared, sig_var, Buf, Const, ConstBuf, Filter,
    GateToTrigRisingEdge, Sig, SigAbs, SigBoxed, SigBoxedVar, SigCtx,
    SigSampleIntoBufT, SigShared, SigT, SigVar, Triggerable,
};
pub mod stereo;
pub use stereo::{Channel, Stereo, StereoPair};
