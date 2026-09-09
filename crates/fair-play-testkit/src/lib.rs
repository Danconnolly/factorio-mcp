//! Disposable Factorio fixture utilities.
//!
//! Production saves and credentials are intentionally outside this crate and
//! outside version control.

#![forbid(unsafe_code)]

/// Artifact directory prefix reserved for disposable Phase-0 runs.
pub const PHASE_ZERO_ARTIFACT_DIRECTORY: &str = "artifacts/phase-0";
