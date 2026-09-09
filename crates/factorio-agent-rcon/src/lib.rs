//! Closed-dispatch RCON transport for the Factorio Fair-Play bridge.
//!
//! This crate accepts only named bridge reads. It intentionally exposes no API
//! for arbitrary RCON commands, Lua, remote targets, or bridge method names.

#![forbid(unsafe_code)]

mod client;
mod command;
mod config;
mod response;

pub use client::RconAdapter;
pub use command::{BridgeCommand, EntityTarget, MAX_LUA_SAFE_INTEGER};
pub use config::{RconConfig, RconTimeouts};
pub use response::{
    ActorCreatedDetail, ActorLifecycleDetail, BridgeActionKind, BridgeActionRecord,
    BridgeEntityRecord, BridgeErrorCode, BridgeResponse, EntityDetail, ObservationAuditDetail,
    ObservationBounds, ObservationResult, RconError, RequestedObservationBounds,
    TransportErrorKind,
};

/// The only Factorio remote interface this crate can target.
pub const BRIDGE_INTERFACE: &str = "factorio_agent_bridge";
