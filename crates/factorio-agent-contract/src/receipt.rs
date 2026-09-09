use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Capability, ContractError, ErrorCode, Provenance};

macro_rules! identifier {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

identifier!(ActionId, "Stable ID for an action lifecycle request.");

/// The complete lifecycle vocabulary. Phase 0 records lifecycle only; it performs no mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionState {
    Accepted,
    Running,
    Succeeded,
    Rejected,
    Cancelled,
    Failed,
}

impl ActionState {
    pub const ALL: [Self; 6] = [
        Self::Accepted,
        Self::Running,
        Self::Succeeded,
        Self::Rejected,
        Self::Cancelled,
        Self::Failed,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

/// Audit event emitted for a bounded read-only result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ObservationRecord {
    pub capability: Capability,
    pub game_tick: u64,
    pub provenance: Provenance,
    pub result_count: u32,
    pub truncated: bool,
}

/// Phase-1 audit records retain read metadata and durable bridge-owned action receipts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditRecord {
    Lifecycle {
        action_id: ActionId,
        state: ActionState,
        game_tick: u64,
    },
    Observation(ObservationRecord),
    Mutation {
        receipt: serde_json::Value,
    },
}

/// The append-only envelope. Only this narrow metadata is hashed into chain links.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AuditEnvelope {
    pub sequence: u64,
    pub previous_digest: String,
    pub record_digest: String,
    pub timestamp: u64,
    pub envelope_digest: String,
    pub record: AuditRecord,
}

/// In-memory builder and verifier for a deterministic SHA-256 audit chain.
#[derive(Clone, Debug, Default)]
pub struct AuditChain {
    next_sequence: u64,
    previous_digest: String,
}

impl AuditChain {
    pub const GENESIS_DIGEST: &'static str =
        "0000000000000000000000000000000000000000000000000000000000000000";

    #[must_use]
    pub fn new() -> Self {
        Self {
            next_sequence: 0,
            previous_digest: Self::GENESIS_DIGEST.to_owned(),
        }
    }

    /// Appends a record at a caller-supplied timestamp.
    ///
    /// # Errors
    ///
    /// Returns `INTERNAL` if the sequence counter is exhausted or canonical JSON fails.
    pub fn append(
        &mut self,
        record: &AuditRecord,
        timestamp: u64,
    ) -> Result<AuditEnvelope, ContractError> {
        let record_digest = digest(record)?;
        let sequence = self.next_sequence;
        let previous_digest = self.previous_digest.clone();
        let envelope_digest = digest(&ChainLink {
            sequence,
            previous_digest: &previous_digest,
            record_digest: &record_digest,
            timestamp,
        })?;
        let envelope = AuditEnvelope {
            sequence,
            previous_digest,
            record_digest,
            timestamp,
            envelope_digest: envelope_digest.clone(),
            record: record.clone(),
        };
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| ContractError::new(ErrorCode::Internal, "audit sequence exhausted"))?;
        self.previous_digest = envelope_digest;
        Ok(envelope)
    }

    /// Restores an append cursor after verifying every persisted envelope.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::verify`] when the persisted chain is invalid.
    pub fn from_verified(envelopes: &[AuditEnvelope]) -> Result<Self, ContractError> {
        Self::verify(envelopes)?;
        let next_sequence = u64::try_from(envelopes.len())
            .map_err(|_| ContractError::new(ErrorCode::Internal, "audit sequence exhausted"))?;
        let previous_digest = envelopes.last().map_or_else(
            || Self::GENESIS_DIGEST.to_owned(),
            |envelope| envelope.envelope_digest.clone(),
        );
        Ok(Self {
            next_sequence,
            previous_digest,
        })
    }

    /// Verifies sequence continuity and each record and envelope digest.
    ///
    /// # Errors
    ///
    /// Returns `INVALID_ARGUMENT` for a malformed chain and `INTERNAL` for serialization failure.
    pub fn verify(envelopes: &[AuditEnvelope]) -> Result<(), ContractError> {
        let mut expected_sequence = 0;
        let mut previous_digest = Self::GENESIS_DIGEST.to_owned();
        for envelope in envelopes {
            if envelope.sequence != expected_sequence {
                return Err(ContractError::new(
                    ErrorCode::InvalidArgument,
                    "audit sequence is discontinuous",
                ));
            }
            if envelope.previous_digest != previous_digest {
                return Err(ContractError::new(
                    ErrorCode::InvalidArgument,
                    "audit previous digest does not match",
                ));
            }
            if envelope.record_digest != digest(&envelope.record)? {
                return Err(ContractError::new(
                    ErrorCode::InvalidArgument,
                    "audit record digest does not match",
                ));
            }
            let expected_digest = digest(&ChainLink {
                sequence: envelope.sequence,
                previous_digest: &envelope.previous_digest,
                record_digest: &envelope.record_digest,
                timestamp: envelope.timestamp,
            })?;
            if envelope.envelope_digest != expected_digest {
                return Err(ContractError::new(
                    ErrorCode::InvalidArgument,
                    "audit envelope digest does not match",
                ));
            }
            expected_sequence = expected_sequence.checked_add(1).ok_or_else(|| {
                ContractError::new(ErrorCode::Internal, "audit sequence exhausted")
            })?;
            previous_digest.clone_from(&envelope.envelope_digest);
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct ChainLink<'a> {
    sequence: u64,
    previous_digest: &'a str,
    record_digest: &'a str,
    timestamp: u64,
}

fn digest(value: &impl Serialize) -> Result<String, ContractError> {
    let canonical = crate::canonical_json(value)?;
    Ok(format!("{:x}", Sha256::digest(canonical.as_bytes())))
}
