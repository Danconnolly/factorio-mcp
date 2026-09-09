use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Public capabilities available during the first Phase-1 movement slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    GetCapabilityContract,
    GetActorStatus,
    ScanLocal,
    ScanCharted,
    GetEntity,
    GetActionRecord,
    WalkTo,
    Stop,
    GetAction,
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
            Self::WalkTo => "walk_to",
            Self::Stop => "stop",
            Self::GetAction => "get_action",
        }
    }

    #[must_use]
    pub const fn phase_one_allow_list() -> &'static [Self] {
        &PHASE_ONE_CAPABILITIES
    }
}

/// The complete Phase-1 walk/stop surface. Additive changes require a policy version change.
pub const PHASE_ONE_CAPABILITIES: [Capability; 9] = [
    Capability::GetCapabilityContract,
    Capability::GetActorStatus,
    Capability::ScanLocal,
    Capability::ScanCharted,
    Capability::GetEntity,
    Capability::GetActionRecord,
    Capability::WalkTo,
    Capability::Stop,
    Capability::GetAction,
];
