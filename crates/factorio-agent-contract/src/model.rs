use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{SCHEMA_VERSION, capability::Capability};

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

identifier!(ActorId, "Bridge-owned virtual actor identity.");
identifier!(ForceId, "Factorio force identity.");
identifier!(ProfileName, "Configured fair-play profile name.");

/// A mod that was enabled when the contract was captured.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EnabledMod {
    pub name: String,
    pub version: String,
}

impl EnabledMod {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

/// Fixed read limits enforced inside the Factorio bridge rather than trusted to clients.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ObservationLimits {
    pub max_radius: u32,
    pub max_result_count: u32,
    pub max_detail_fields: u32,
    pub max_payload_bytes: u32,
}

impl ObservationLimits {
    #[must_use]
    pub const fn new(
        max_radius: u32,
        max_result_count: u32,
        max_detail_fields: u32,
        max_payload_bytes: u32,
    ) -> Self {
        Self {
            max_radius,
            max_result_count,
            max_detail_fields,
            max_payload_bytes,
        }
    }
}

/// How an observation request is ordered relative to the Factorio tick stream.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SchedulingSemantics {
    pub execution: String,
    pub observation_tick: String,
    pub request_order: String,
}

impl SchedulingSemantics {
    #[must_use]
    pub fn phase_zero() -> Self {
        Self {
            execution: "read_only".to_owned(),
            observation_tick: "bridge_tick_snapshot".to_owned(),
            request_order: "serialized".to_owned(),
        }
    }

    #[must_use]
    pub fn phase_one_walk_stop() -> Self {
        Self {
            execution: "tick_driven_character_input".to_owned(),
            observation_tick: "bridge_tick_snapshot".to_owned(),
            request_order: "one_active_action_serialized_at_next_game_tick".to_owned(),
        }
    }
}

/// The versioned discovery response required before a client can issue tools.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityContract {
    pub schema_version: String,
    pub fair_play_policy_version: String,
    pub bridge_build: String,
    pub factorio_build: String,
    pub enabled_mods: Vec<EnabledMod>,
    pub actor_id: ActorId,
    pub force_id: ForceId,
    pub profile_name: ProfileName,
    pub observation_limits: ObservationLimits,
    pub scheduling: SchedulingSemantics,
    /// The complete fixed Phase-1 manifest, in protocol dispatch order.
    pub capabilities: PhaseOneCapabilities,
    /// Provenance vocabulary this bridge can attach to observations.
    pub supported_provenance: Vec<Provenance>,
    pub current_game_tick: u64,
}

/// The only capability manifest permitted by the Phase-1 walk/stop protocol.
///
/// The tuple schema and deserialization check reject additions, omissions,
/// reordering, and duplicates rather than merely accepting six capabilities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PhaseOneCapabilities([Capability; 9]);

impl PhaseOneCapabilities {
    #[must_use]
    pub const fn exact() -> Self {
        Self(crate::capability::PHASE_ONE_CAPABILITIES)
    }

    #[must_use]
    pub const fn as_array(&self) -> &[Capability; 9] {
        &self.0
    }
}

impl Default for PhaseOneCapabilities {
    fn default() -> Self {
        Self::exact()
    }
}

impl<'de> Deserialize<'de> for PhaseOneCapabilities {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let capabilities = <[Capability; 9]>::deserialize(deserializer)?;
        if capabilities != crate::capability::PHASE_ONE_CAPABILITIES {
            return Err(de::Error::custom(
                "capabilities must be the exact Phase-1 manifest",
            ));
        }
        Ok(Self(capabilities))
    }
}

impl JsonSchema for PhaseOneCapabilities {
    fn schema_name() -> Cow<'static, str> {
        "PhaseOneCapabilities".into()
    }

    fn inline_schema() -> bool {
        true
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "array",
            "prefixItems": [
                { "const": "get_capability_contract" },
                { "const": "get_actor_status" },
                { "const": "scan_local" },
                { "const": "scan_charted" },
                { "const": "get_entity" },
                { "const": "get_action_record" },
                { "const": "walk_to" },
                { "const": "stop" },
                { "const": "get_action" }
            ],
            "items": false,
            "minItems": 9,
            "maxItems": 9
        })
    }
}

impl CapabilityContract {
    #[must_use]
    pub fn phase_one_capabilities() -> &'static [Capability] {
        Capability::phase_one_allow_list()
    }

    #[must_use]
    pub fn schema_version_matches_build(&self) -> bool {
        self.schema_version == SCHEMA_VERSION
    }
}

/// Why an observation is allowed. Clients must retain this with every record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    CharacterLocal,
    ForceCharted,
    PermittedDirectInteraction,
    ForceStatistics,
}

/// Entity material exposed by a bounded observation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EntityRecord {
    pub unit_number: u64,
    pub name: String,
    pub position: Position,
    pub provenance: Provenance,
}

/// Tile material exposed by a bounded observation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TileRecord {
    pub name: String,
    pub position: Position,
    pub provenance: Provenance,
}

/// Resource material exposed by a bounded observation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResourceRecord {
    pub name: String,
    pub amount: u64,
    pub position: Position,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Stable lifecycle state emitted by the read-only actor endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActorLifecycleState {
    Ready,
    Unavailable,
}

/// A bounded inventory entry. The bridge sorts entries by item name.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InventoryItemSummary {
    pub name: String,
    pub count: u64,
}

/// Inventory material emitted without exposing inventory slots or mutable handles.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InventorySummary {
    pub total: u64,
    pub items: Vec<InventoryItemSummary>,
    pub truncated: bool,
}

/// Surface identity kept with the actor's immutable persisted unit number.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SurfaceIdentity {
    pub index: u32,
    pub name: String,
}

/// Force identity carried separately from the configured actor ID.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ForceIdentity {
    pub name: String,
}

/// Versions reported by the bridge with every actor status response.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BridgeVersions {
    pub build: String,
    pub lifecycle_schema_version: u32,
    pub policy_version: String,
}

/// Typed, read-only lifecycle/status payload for the one bridge-owned actor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ActorStatus {
    pub state: ActorLifecycleState,
    pub actor_id: ActorId,
    pub unit_number: u64,
    pub surface: SurfaceIdentity,
    pub force: ForceIdentity,
    pub position: Position,
    pub health: f64,
    pub inventory: InventorySummary,
    pub tick: u64,
    pub bridge: BridgeVersions,
}
