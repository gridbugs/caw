mod arith;
mod frame_sig;
pub mod frame_sig_ops;
mod sig;
pub mod sig_ops;
pub use frame_sig::{
    frame_sig_shared, FrameSig, FrameSigEdges, FrameSigShared, FrameSigT,
    Triggerable,
};
pub use sig::{
    sig_shared, Buf, ConstBuf, Filter, GateToTrigRisingEdge, Sig, SigAbs,
    SigBoxed, SigCtx, SigSampleIntoBufT, SigShared, SigT,
};
pub mod stereo;
pub use stereo::{Channel, Stereo};
