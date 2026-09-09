//! Versioned, transport-independent contracts for Factorio Fair-Play MCP.
//!
//! Phase 1 adds a deliberately small tick-driven walking surface to the
//! read-only actor and bounded-observation foundation.

#![forbid(unsafe_code)]

pub mod capability;
pub mod error;
pub mod model;
pub mod receipt;
pub mod schema;

pub use capability::{Capability, PHASE_ONE_CAPABILITIES};
pub use error::{ContractError, ErrorCode};
pub use model::{
    ActorId, ActorLifecycleState, ActorStatus, BridgeVersions, CapabilityContract, EnabledMod,
    EntityRecord, ForceId, ForceIdentity, InventoryItemSummary, InventorySummary,
    ObservationLimits, PhaseOneCapabilities, Position, ProfileName, Provenance, ResourceRecord,
    SchedulingSemantics, SurfaceIdentity, TileRecord,
};
pub use receipt::{
    ActionId, ActionState, AuditChain, AuditEnvelope, AuditRecord, ObservationRecord,
};
pub use schema::{
    actor_status_schema, audit_envelope_schema, audit_schema, canonical_json, capability_schema,
    entity_record_schema, resource_record_schema, tile_record_schema,
};

/// Contract schema version implemented by this build.
pub const SCHEMA_VERSION: &str = "0.2.0";
