mod arith;
mod frame_sig;
pub mod frame_sig_ops;
mod sig;
pub mod sig_ops;
pub use frame_sig::{
    frame_sig_shared, frame_sig_var, FrameSig, FrameSigEdges, FrameSigShared,
    FrameSigT, FrameSigVar, Triggerable,
};
pub use sig::{
    sig_shared, Buf, ConstBuf, Filter, GateToTrigRisingEdge, Sig, SigAbs,
    SigBoxed, SigCtx, SigSampleIntoBufT, SigShared, SigT,
};
pub mod stereo;
pub use stereo::{Channel, Stereo, StereoPair};
