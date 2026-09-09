use factorio_agent_contract::{Position, canonical_json};
use serde::Serialize;

use crate::{
    BRIDGE_INTERFACE,
    response::{BridgeErrorCode, RconError},
};

const QUERY_METHOD: &str = "query";
const COMMAND_METHOD: &str = "command";
const MAX_COMMAND_BYTES: usize = 4 * 1024;
/// Largest integer Lua can represent exactly when received through JSON/RCON.
pub const MAX_LUA_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;

/// Closed set of approved read-only bridge calls.
#[derive(Clone, Debug, PartialEq)]
pub enum BridgeCommand {
    GetCapabilityContract,
    GetActorStatus,
    ScanLocal { radius: u32 },
    ScanCharted { center: Position, radius: u32 },
    GetEntity { target: EntityTarget },
    GetActionRecord { sequence: u64 },
    WalkTo { action_id: String, target: Position },
    Stop { action_id: String },
    GetAction { action_id: String },
}

/// The only two policy-approved ways to identify an entity.
#[derive(Clone, Debug, PartialEq)]
pub enum EntityTarget {
    IssuedId(String),
    Position(Position),
}

impl BridgeCommand {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn lua_command(&self) -> Result<String, RconError> {
        if let Self::GetActionRecord { sequence } = self
            && *sequence > MAX_LUA_SAFE_INTEGER
        {
            return Err(RconError::Bridge {
                code: BridgeErrorCode::InvalidArgument,
            });
        }
        let request = match self {
            Self::GetCapabilityContract => QueryRequest {
                name: "get_capability_contract",
                radius: None,
                center: None,
                entity_id: None,
                position: None,
                sequence: None,
                action_id: None,
                target: None,
            },
            Self::GetActorStatus => QueryRequest {
                name: "get_actor_status",
                radius: None,
                center: None,
                entity_id: None,
                position: None,
                sequence: None,
                action_id: None,
                target: None,
            },
            Self::ScanLocal { radius } => QueryRequest {
                name: "scan_local",
                radius: Some(*radius),
                center: None,
                entity_id: None,
                position: None,
                sequence: None,
                action_id: None,
                target: None,
            },
            Self::ScanCharted { center, radius } => QueryRequest {
                name: "scan_charted",
                radius: Some(*radius),
                center: Some(*center),
                entity_id: None,
                position: None,
                sequence: None,
                action_id: None,
                target: None,
            },
            Self::GetEntity { target } => match target {
                EntityTarget::IssuedId(entity_id) => QueryRequest {
                    name: "get_entity",
                    radius: None,
                    center: None,
                    entity_id: Some(entity_id),
                    position: None,
                    sequence: None,
                    action_id: None,
                    target: None,
                },
                EntityTarget::Position(position) => QueryRequest {
                    name: "get_entity",
                    radius: None,
                    center: None,
                    entity_id: None,
                    position: Some(*position),
                    sequence: None,
                    action_id: None,
                    target: None,
                },
            },
            Self::GetActionRecord { sequence } => QueryRequest {
                name: "get_action_record",
                radius: None,
                center: None,
                entity_id: None,
                position: None,
                sequence: Some(*sequence),
                action_id: None,
                target: None,
            },
            Self::WalkTo { action_id, target } => QueryRequest {
                name: "walk_to",
                radius: None,
                center: None,
                entity_id: None,
                position: None,
                sequence: None,
                action_id: Some(action_id),
                target: Some(*target),
            },
            Self::Stop { action_id } => QueryRequest {
                name: "stop",
                radius: None,
                center: None,
                entity_id: None,
                position: None,
                sequence: None,
                action_id: Some(action_id),
                target: None,
            },
            Self::GetAction { action_id } => QueryRequest {
                name: "get_action",
                radius: None,
                center: None,
                entity_id: None,
                position: None,
                sequence: None,
                action_id: Some(action_id),
                target: None,
            },
        };
        let canonical_request = canonical_json(&request).map_err(|_| RconError::Bridge {
            code: BridgeErrorCode::SerializationFailure,
        })?;
        // The request JSON is encoded once more as one Lua string literal. No request field can
        // become Lua syntax, an RCON command, a remote interface, or a method name.
        let lua_literal =
            serde_json::to_string(&canonical_request).map_err(|_| RconError::Bridge {
                code: BridgeErrorCode::SerializationFailure,
            })?;
        let command = format!(
            "/c rcon.print(helpers.table_to_json(remote.call(\"{BRIDGE_INTERFACE}\",\"{}\",helpers.json_to_table({lua_literal}))))",
            self.remote_method(),
        );
        if command.len() > MAX_COMMAND_BYTES {
            return Err(RconError::Bridge {
                code: BridgeErrorCode::RequestTooLarge,
            });
        }
        Ok(command)
    }

    pub(crate) const fn is_mutation(&self) -> bool {
        matches!(self, Self::WalkTo { .. } | Self::Stop { .. })
    }

    const fn remote_method(&self) -> &'static str {
        match self {
            Self::WalkTo { .. } | Self::Stop { .. } | Self::GetAction { .. } => COMMAND_METHOD,
            _ => QUERY_METHOD,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "name", rename_all = "snake_case")]
struct QueryRequest<'a> {
    name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    radius: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<Position>,
}

#[cfg(test)]
mod tests {
    use factorio_agent_contract::Position;

    use super::{BridgeCommand, EntityTarget, MAX_LUA_SAFE_INTEGER};

    #[test]
    fn every_permitted_lua_request_has_a_closed_rust_variant() {
        let commands = [
            BridgeCommand::GetActorStatus,
            BridgeCommand::GetCapabilityContract,
            BridgeCommand::ScanLocal { radius: 1 },
            BridgeCommand::ScanCharted {
                center: Position { x: 1.0, y: 2.0 },
                radius: 1,
            },
            BridgeCommand::GetEntity {
                target: EntityTarget::Position(Position { x: 1.0, y: 2.0 }),
            },
            BridgeCommand::GetActionRecord { sequence: 0 },
        ];
        let names: Vec<_> = commands
            .iter()
            .map(|command| {
                let command = command.lua_command().unwrap();
                let literal = command
                    .split("helpers.json_to_table(")
                    .nth(1)
                    .unwrap()
                    .strip_suffix("))))")
                    .unwrap();
                let request: String = serde_json::from_str(literal).unwrap();
                serde_json::from_str::<serde_json::Value>(&request).unwrap()["name"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(
            names,
            [
                "get_actor_status",
                "get_capability_contract",
                "scan_local",
                "scan_charted",
                "get_entity",
                "get_action_record"
            ]
        );
    }

    #[test]
    fn action_record_sequence_never_exceeds_lua_exact_integer_range() {
        assert!(
            BridgeCommand::GetActionRecord {
                sequence: MAX_LUA_SAFE_INTEGER
            }
            .lua_command()
            .is_ok()
        );
        assert!(
            BridgeCommand::GetActionRecord {
                sequence: MAX_LUA_SAFE_INTEGER + 1
            }
            .lua_command()
            .is_err()
        );
    }
}
