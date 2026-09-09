use schemars::{Schema, schema_for};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    ActorStatus, AuditEnvelope, AuditRecord, CapabilityContract, ContractError, EntityRecord,
    ErrorCode, ResourceRecord, TileRecord,
};

/// Schemars schema for the discovery contract.
#[must_use]
pub fn capability_schema() -> Schema {
    schema_for!(CapabilityContract)
}

/// Schemars schema for the typed, read-only actor lifecycle response.
#[must_use]
pub fn actor_status_schema() -> Schema {
    schema_for!(ActorStatus)
}

/// Schemars schema for the Phase-0 audit record, deliberately excluding future receipts.
#[must_use]
pub fn audit_schema() -> Schema {
    schema_for!(AuditRecord)
}

/// Schemars schema for the append-only audit envelope and its chain metadata.
#[must_use]
pub fn audit_envelope_schema() -> Schema {
    schema_for!(AuditEnvelope)
}

/// Schemars schema for an entity observation record.
#[must_use]
pub fn entity_record_schema() -> Schema {
    schema_for!(EntityRecord)
}

/// Schemars schema for a tile observation record.
#[must_use]
pub fn tile_record_schema() -> Schema {
    schema_for!(TileRecord)
}

/// Schemars schema for a resource observation record.
#[must_use]
pub fn resource_record_schema() -> Schema {
    schema_for!(ResourceRecord)
}

/// Stable JSON serialization: object keys are recursively sorted and no whitespace is emitted.
///
/// # Errors
///
/// Returns `INTERNAL` if the value cannot be represented as JSON.
pub fn canonical_json(value: &impl Serialize) -> Result<String, ContractError> {
    let value = serde_json::to_value(value)
        .map_err(|error| ContractError::new(ErrorCode::Internal, error.to_string()))?;
    serde_json::to_string(&canonicalize(value))
        .map_err(|error| ContractError::new(ErrorCode::Internal, error.to_string()))
}

fn canonicalize(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.into_iter().map(canonicalize).collect()),
        Value::Object(values) => {
            let mut entries: Vec<_> = values.into_iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = Map::new();
            for (key, value) in entries {
                sorted.insert(key, canonicalize(value));
            }
            Value::Object(sorted)
        }
        primitive => primitive,
    }
}
