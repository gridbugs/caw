mod arith;
mod sig;
pub mod sig_ops;
pub use sig::{
    sig_option_first_some, sig_shared, sig_var, Buf, ConstBuf, Filter,
    GateToTrigRisingEdge, Sig, SigAbs, SigBoxed, SigCtx, SigSampleIntoBufT,
    SigShared, SigT, SigVar, Triggerable,
};
pub mod stereo;
pub use stereo::{Channel, Stereo, StereoPair};
