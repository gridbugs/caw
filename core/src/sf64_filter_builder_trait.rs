use crate::{builder::filter::*, signal::Sf64};

/// A trait for allowing filters to be applied by calling methods directly rather than via the
/// `filter` method.
pub trait Sf64FilterBuilderTrait {
    fn low_pass_butterworth(&self, cutoff_hz: impl Into<Sf64>) -> LowPassButterworthBuilder;
}
