use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable code intended for automation. Human context stays in `detail`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum ErrorCode {
    #[serde(rename = "INVALID_ARGUMENT")]
    InvalidArgument,
    #[serde(rename = "ACTOR_UNAVAILABLE")]
    ActorUnavailable,
    #[serde(rename = "OUT_OF_POLICY")]
    OutOfPolicy,
    #[serde(rename = "UNCHARTED")]
    Uncharted,
    #[serde(rename = "NOT_FOUND")]
    NotFound,
    #[serde(rename = "BRIDGE_UNAVAILABLE")]
    BridgeUnavailable,
    #[serde(rename = "VERSION_MISMATCH")]
    VersionMismatch,
    #[serde(rename = "INTERNAL")]
    Internal,
}

/// A machine-readable contract error with separately serialised human detail.
#[derive(Clone, Debug, Eq, PartialEq, Error, Serialize, Deserialize, JsonSchema)]
#[error("{code:?}: {detail}")]
pub struct ContractError {
    pub code: ErrorCode,
    pub detail: String,
}

impl ContractError {
    pub fn new(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}
