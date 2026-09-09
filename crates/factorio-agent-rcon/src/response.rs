use factorio_agent_contract::{
    ActorStatus, CapabilityContract, Position, Provenance, ResourceRecord, TileRecord,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Structured adapter failures with no credential-bearing fields.
#[derive(Debug, Error)]
pub enum RconError {
    #[error("RCON configuration error: {reason}")]
    Configuration { reason: &'static str },
    #[error("RCON transport error: {kind}")]
    Transport { kind: TransportErrorKind },
    #[error("bridge error: {code}")]
    Bridge { code: BridgeErrorCode },
    #[error("bridge metadata mismatch")]
    VersionMismatch,
}

impl RconError {
    #[must_use]
    pub fn configuration(reason: &'static str) -> Self {
        Self::Configuration { reason }
    }
}

/// Closed vocabulary for bridge and adapter failures exposed to callers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum BridgeErrorCode {
    #[error("INVALID_ARGUMENT")]
    InvalidArgument,
    #[error("ACTOR_UNAVAILABLE")]
    ActorUnavailable,
    #[error("OUT_OF_POLICY")]
    OutOfPolicy,
    #[error("UNCHARTED")]
    Uncharted,
    #[error("NOT_FOUND")]
    NotFound,
    #[error("BRIDGE_UNAVAILABLE")]
    BridgeUnavailable,
    #[error("VERSION_MISMATCH")]
    VersionMismatch,
    #[error("INTERNAL")]
    Internal,
    #[error("MALFORMED_RESPONSE")]
    MalformedResponse,
    #[error("UNKNOWN_REMOTE_ERROR")]
    UnknownRemoteError,
    #[error("SERIALIZATION_FAILURE")]
    SerializationFailure,
    #[error("REQUEST_TOO_LARGE")]
    RequestTooLarge,
    #[error("RESPONSE_TOO_LARGE")]
    ResponseTooLarge,
}

impl BridgeErrorCode {
    fn from_remote(code: &str) -> Self {
        match code {
            "INVALID_ARGUMENT" => Self::InvalidArgument,
            "ACTOR_UNAVAILABLE" => Self::ActorUnavailable,
            "OUT_OF_POLICY" => Self::OutOfPolicy,
            "UNCHARTED" => Self::Uncharted,
            "NOT_FOUND" => Self::NotFound,
            "BRIDGE_UNAVAILABLE" => Self::BridgeUnavailable,
            "VERSION_MISMATCH" => Self::VersionMismatch,
            "INTERNAL" => Self::Internal,
            _ => Self::UnknownRemoteError,
        }
    }
}

/// Sanitized transport failure classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum TransportErrorKind {
    #[error("connection failed")]
    Connection,
    #[error("authentication failed")]
    Authentication,
    #[error("timed out")]
    Timeout,
    #[error("connection lost")]
    ConnectionLost,
    #[error("invalid RCON framing")]
    Framing,
}

/// Bounds supplied by a client; local scans intentionally omit their center.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct RequestedObservationBounds {
    pub center: Option<Position>,
    pub radius: u32,
}
/// Bounds enforced by the bridge after resolving the actor and policy.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ObservationBounds {
    pub center: Position,
    pub radius: u32,
}
/// Entity material as issued by the Lua bridge, including its opaque permitted ID.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct BridgeEntityRecord {
    pub entity_id: Option<String>,
    pub name: String,
    pub position: Position,
    pub provenance: Provenance,
    pub detail: Vec<EntityDetail>,
}
/// One bounded, name/value entity detail item.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct EntityDetail {
    pub name: String,
    pub value: serde_json::Value,
}
/// Typed result from either bounded scan bridge endpoint.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ObservationResult {
    pub requested_bounds: RequestedObservationBounds,
    pub effective_bounds: ObservationBounds,
    pub provenance: Provenance,
    pub entities: Vec<BridgeEntityRecord>,
    pub resources: Vec<ResourceRecord>,
    pub tiles: Vec<TileRecord>,
    pub result_count: u32,
    pub partial: bool,
    pub truncated: bool,
    pub payload_bytes: u32,
}
/// Typed audit record retained by the production Lua bridge.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct BridgeActionRecord {
    pub tick: u64,
    #[serde(flatten)]
    pub action: BridgeActionKind,
}
/// Closed audit vocabulary currently emitted by `audit.lua` and `actor.lua`.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum BridgeActionKind {
    Observation(ObservationAuditDetail),
    ActorCreated(ActorCreatedDetail),
    ActorLifecycle(ActorLifecycleDetail),
}
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ObservationAuditDetail {
    pub capability: String,
    pub decision: String,
    pub requested_bounds: Option<RequestedObservationBounds>,
    pub effective_bounds: Option<ObservationBounds>,
    pub provenance: Option<Provenance>,
    pub result_count: u32,
    pub truncated: bool,
}
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ActorCreatedDetail {
    pub actor_id: String,
    pub unit_number: u64,
}
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ActorLifecycleDetail {
    pub state: String,
}

/// A typed result from the fixed bridge query endpoint.
#[derive(Clone, Debug, PartialEq)]
pub enum BridgeResponse {
    CapabilityContract(CapabilityContract),
    ActorStatus(ActorStatus),
    ScanLocal(ObservationResult),
    ScanCharted(ObservationResult),
    Entity(BridgeEntityRecord),
    ActionRecord(BridgeActionRecord),
}

#[derive(Deserialize)]
pub(crate) struct Envelope<T> {
    ok: bool,
    result: Option<T>,
    error: Option<BridgeFailure>,
}
#[derive(Deserialize)]
pub(crate) struct BridgeFailure {
    code: String,
}
pub(crate) fn decode_contract(payload: &str) -> Result<CapabilityContract, RconError> {
    decode(payload)
}
pub(crate) fn decode_actor_status(payload: &str) -> Result<ActorStatus, RconError> {
    decode(payload)
}
pub(crate) fn decode_observation(payload: &str) -> Result<ObservationResult, RconError> {
    decode(payload)
}
pub(crate) fn decode_entity(payload: &str) -> Result<BridgeEntityRecord, RconError> {
    decode(payload)
}
pub(crate) fn decode_action_record(payload: &str) -> Result<BridgeActionRecord, RconError> {
    decode(payload)
}

fn decode<T: for<'de> Deserialize<'de>>(payload: &str) -> Result<T, RconError> {
    let envelope: Envelope<T> = serde_json::from_str(payload).map_err(|_| RconError::Bridge {
        code: BridgeErrorCode::MalformedResponse,
    })?;
    if !envelope.ok {
        return Err(RconError::Bridge {
            code: envelope
                .error
                .map_or(BridgeErrorCode::MalformedResponse, |failure| {
                    BridgeErrorCode::from_remote(&failure.code)
                }),
        });
    }
    envelope.result.ok_or(RconError::Bridge {
        code: BridgeErrorCode::MalformedResponse,
    })
}

#[cfg(test)]
mod tests {
    use super::{BridgeActionKind, decode_action_record};

    #[test]
    fn decodes_the_lua_action_record_shape() {
        let payload = r#"{"ok":true,"result":{"kind":"actor_created","detail":{"actor_id":"alfred","unit_number":9},"tick":42}}"#;
        let record = decode_action_record(payload).unwrap();
        assert_eq!(record.tick, 42);
        assert!(matches!(record.action, BridgeActionKind::ActorCreated(_)));
    }
}
