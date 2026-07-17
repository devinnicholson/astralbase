//! Bounded, in-memory retrograde exploration for constrained chess research.
//!
//! The reusable engine is available with no optional features. The
//! `partizan-dataset` default feature retains the historical dataset-generation
//! surface used by the Partizan research repository.

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "partizan-dataset")]
#[cfg_attr(docsrs, doc(cfg(feature = "partizan-dataset")))]
pub mod artifact;
#[cfg(feature = "partizan-dataset")]
#[cfg_attr(docsrs, doc(cfg(feature = "partizan-dataset")))]
pub mod dataset_label;
#[cfg(feature = "partizan-dataset")]
#[cfg_attr(docsrs, doc(cfg(feature = "partizan-dataset")))]
pub mod domain;
pub mod engine;
pub mod retrograde;

pub use engine::{GameValue, ProbeResult, RetrogradeEngine};
