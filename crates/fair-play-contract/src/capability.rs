use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Public read-only capabilities available during Phase 0.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    GetCapabilityContract,
    GetActorStatus,
    ScanLocal,
    ScanCharted,
    GetEntity,
    GetActionRecord,
}

impl Capability {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GetCapabilityContract => "get_capability_contract",
            Self::GetActorStatus => "get_actor_status",
            Self::ScanLocal => "scan_local",
            Self::ScanCharted => "scan_charted",
            Self::GetEntity => "get_entity",
            Self::GetActionRecord => "get_action_record",
        }
    }

    #[must_use]
    pub const fn phase_zero_allow_list() -> &'static [Self] {
        &PHASE_ZERO_CAPABILITIES
    }
}

/// The complete Phase-0 surface. Additive capability changes require a policy version change.
pub const PHASE_ZERO_CAPABILITIES: [Capability; 6] = [
    Capability::GetCapabilityContract,
    Capability::GetActorStatus,
    Capability::ScanLocal,
    Capability::ScanCharted,
    Capability::GetEntity,
    Capability::GetActionRecord,
];
