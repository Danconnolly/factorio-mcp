//! Strict stdio MCP service for the Phase-0 read-only capability profile.

#![forbid(unsafe_code)]

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use factorio_agent_contract::{
    ActionId, ActionState, AuditChain, AuditEnvelope, AuditRecord, Capability, CapabilityContract,
    ContractError, ErrorCode, ObservationRecord, Position,
};
use factorio_agent_rcon::{
    BridgeCommand, BridgeErrorCode, BridgeResponse, EntityTarget, RconAdapter, RconError,
    TransportErrorKind,
};
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

const MAX_HOST_RADIUS: u32 = 256;
const MAX_ENTITY_ID_BYTES: usize = 128;

/// Durable, append-only audit log for this MCP process.
pub struct AuditSink {
    path: PathBuf,
    lock_path: PathBuf,
    state: Mutex<AuditState>,
}

struct AuditState {
    chain: AuditChain,
    poisoned: bool,
}

impl AuditSink {
    /// Opens a JSONL audit log only after validating every existing record and chain link.
    ///
    /// # Errors
    ///
    /// Returns an error when the log is unreadable, malformed, truncated, or its chain is broken.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ContractError> {
        let path = path.into();
        let records = load_audit_log(&path)?;
        let chain = AuditChain::from_verified(&records)?;
        Ok(Self {
            lock_path: audit_lock_path(&path),
            path,
            state: Mutex::new(AuditState {
                chain,
                poisoned: false,
            }),
        })
    }

    /// Appends one canonical audit envelope and syncs it before advancing the local chain cursor.
    ///
    /// Every cooperating process takes the same exclusive lock, then re-verifies the persisted
    /// chain. Any failure after writing begins leaves durability ambiguous, so this sink stops.
    ///
    /// # Errors
    ///
    /// Returns an error when the append-only log cannot be serialized, written, or synced.
    pub fn append(&self, record: &AuditRecord) -> Result<AuditEnvelope, ContractError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ContractError::new(ErrorCode::Internal, "system clock is before epoch"))?
            .as_secs();
        let mut state = self
            .state
            .lock()
            .map_err(|_| ContractError::new(ErrorCode::Internal, "audit lock poisoned"))?;
        if state.poisoned {
            return Err(ContractError::new(
                ErrorCode::Internal,
                "audit sink is poisoned",
            ));
        }

        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&self.lock_path)
            .map_err(|_| ContractError::new(ErrorCode::Internal, "audit lock is unavailable"))?;
        fs2::FileExt::lock_exclusive(&lock_file)
            .map_err(|_| ContractError::new(ErrorCode::Internal, "audit lock is unavailable"))?;

        let result = (|| {
            let persisted = load_audit_log(&self.path)?;
            let mut candidate = AuditChain::from_verified(&persisted)?;
            let envelope = candidate.append(record, timestamp)?;
            let line = factorio_agent_contract::canonical_json(&envelope)?;
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .map_err(|_| ContractError::new(ErrorCode::Internal, "audit log is unavailable"))?;
            if file
                .write_all(line.as_bytes())
                .and_then(|()| file.write_all(b"\n"))
                .and_then(|()| file.sync_data())
                .is_err()
            {
                state.poisoned = true;
                return Err(ContractError::new(
                    ErrorCode::Internal,
                    "audit log append failed; sink stopped",
                ));
            }
            state.chain = candidate;
            Ok(envelope)
        })();

        let _ = fs2::FileExt::unlock(&lock_file);
        result
    }
}

fn audit_lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.to_path_buf();
    let extension = path.extension().map_or_else(
        || "lock".into(),
        |extension| {
            let mut extension = extension.to_os_string();
            extension.push(".lock");
            extension
        },
    );
    lock_path.set_extension(extension);
    lock_path
}

fn load_audit_log(path: &Path) -> Result<Vec<AuditEnvelope>, ContractError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let metadata = fs::metadata(path)
        .map_err(|_| ContractError::new(ErrorCode::Internal, "audit log is unreadable"))?;
    if !metadata.file_type().is_file() {
        return Err(ContractError::new(
            ErrorCode::InvalidArgument,
            "audit log must be a regular file",
        ));
    }
    let bytes = fs::read(path)
        .map_err(|_| ContractError::new(ErrorCode::Internal, "audit log is unreadable"))?;
    if !bytes.is_empty() && !bytes.ends_with(b"\n") {
        return Err(ContractError::new(
            ErrorCode::InvalidArgument,
            "audit log is truncated",
        ));
    }
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_slice(line).map_err(|_| {
                ContractError::new(
                    ErrorCode::InvalidArgument,
                    "audit log contains malformed JSON",
                )
            })
        })
        .collect()
}

/// A connected bridge and a verified audit log. No route exists outside the six capabilities.
pub struct PhaseZeroServer {
    bridge: RconAdapter,
    audit: AuditSink,
    contract: CapabilityContract,
    tool_router: ToolRouter<Self>,
}

impl PhaseZeroServer {
    #[must_use]
    pub fn new(bridge: RconAdapter, audit: AuditSink, contract: CapabilityContract) -> Self {
        Self {
            bridge,
            audit,
            contract,
            tool_router: Self::tool_router(),
        }
    }

    async fn call(&self, command: BridgeCommand) -> Result<BridgeResponse, ContractError> {
        self.bridge.call(command).await.map_err(map_bridge_error)
    }

    fn result<T: serde::Serialize>(value: &T) -> CallToolResult {
        match factorio_agent_contract::canonical_json(value) {
            Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
            Err(error) => error_result(error),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn audited_result<T: serde::Serialize>(
        &self,
        value: &T,
        record: AuditRecord,
    ) -> CallToolResult {
        if let Err(error) = self.audit.append(&record) {
            return error_result(error);
        }
        Self::result(value)
    }
}

#[tool_router]
impl PhaseZeroServer {
    #[tool(
        name = "get_capability_contract",
        description = "Read the bridge-owned Phase-0 contract.",
        annotations(read_only_hint = true)
    )]
    async fn get_capability_contract(&self, _: Parameters<EmptyInput>) -> CallToolResult {
        match self.call(BridgeCommand::GetCapabilityContract).await {
            Ok(BridgeResponse::CapabilityContract(contract)) => self.audited_result(
                &contract,
                lifecycle_record("get_capability_contract", contract.current_game_tick),
            ),
            Ok(_) => error_result(internal_mismatch()),
            Err(error) => error_result(error),
        }
    }

    #[tool(
        name = "get_actor_status",
        description = "Read status for only the bridge-owned actor.",
        annotations(read_only_hint = true)
    )]
    async fn get_actor_status(&self, _: Parameters<EmptyInput>) -> CallToolResult {
        match self.call(BridgeCommand::GetActorStatus).await {
            Ok(BridgeResponse::ActorStatus(status)) => {
                self.audited_result(&status, lifecycle_record("get_actor_status", status.tick))
            }
            Ok(_) => error_result(internal_mismatch()),
            Err(error) => error_result(error),
        }
    }

    #[tool(
        name = "scan_local",
        description = "Read a bounded observation around the bridge actor.",
        annotations(read_only_hint = true)
    )]
    async fn scan_local(&self, Parameters(input): Parameters<RadiusInput>) -> CallToolResult {
        if let Err(error) = validate_radius(input.radius) {
            return error_result(error);
        }
        match self
            .call(BridgeCommand::ScanLocal {
                radius: input.radius,
            })
            .await
        {
            Ok(BridgeResponse::ScanLocal(result)) => {
                self.audited_result(&result, observation_record(Capability::ScanLocal, &result))
            }
            Ok(_) => error_result(internal_mismatch()),
            Err(error) => error_result(error),
        }
    }

    #[tool(
        name = "scan_charted",
        description = "Read a bounded force-charted observation.",
        annotations(read_only_hint = true)
    )]
    async fn scan_charted(
        &self,
        Parameters(input): Parameters<ChartedScanInput>,
    ) -> CallToolResult {
        if let Err(error) =
            validate_radius(input.radius).and_then(|()| validate_position(input.center))
        {
            return error_result(error);
        }
        match self
            .call(BridgeCommand::ScanCharted {
                center: input.center,
                radius: input.radius,
            })
            .await
        {
            Ok(BridgeResponse::ScanCharted(result)) => self.audited_result(
                &result,
                observation_record(Capability::ScanCharted, &result),
            ),
            Ok(_) => error_result(internal_mismatch()),
            Err(error) => error_result(error),
        }
    }

    #[tool(
        name = "get_entity",
        description = "Read one policy-permitted entity by issued ID or position.",
        annotations(read_only_hint = true)
    )]
    async fn get_entity(&self, Parameters(input): Parameters<EntityInput>) -> CallToolResult {
        let target = match input.target() {
            Ok(target) => target,
            Err(error) => return error_result(error),
        };
        match self.call(BridgeCommand::GetEntity { target }).await {
            Ok(BridgeResponse::Entity(entity)) => self.audited_result(
                &entity,
                AuditRecord::Observation(ObservationRecord {
                    capability: Capability::GetEntity,
                    game_tick: 0,
                    provenance: entity.provenance,
                    result_count: 1,
                    truncated: false,
                }),
            ),
            Ok(_) => error_result(internal_mismatch()),
            Err(error) => error_result(error),
        }
    }

    #[tool(
        name = "get_action_record",
        description = "Read one retained Phase-0 bridge lifecycle or observation record.",
        annotations(read_only_hint = true)
    )]
    async fn get_action_record(
        &self,
        Parameters(input): Parameters<ActionRecordInput>,
    ) -> CallToolResult {
        if input.sequence > factorio_agent_rcon::MAX_LUA_SAFE_INTEGER {
            return error_result(ContractError::new(
                ErrorCode::InvalidArgument,
                "sequence exceeds the Lua exact integer range",
            ));
        }
        match self
            .call(BridgeCommand::GetActionRecord {
                sequence: input.sequence,
            })
            .await
        {
            Ok(BridgeResponse::ActionRecord(record)) => Self::result(&record),
            Ok(_) => error_result(internal_mismatch()),
            Err(error) => error_result(error),
        }
    }
}

#[allow(clippy::unused_async_trait_impl)]
#[tool_handler(router = self.tool_router, name = "factorio-agent-mcp", version = "0.1.0", instructions = "Phase-0 read-only Factorio capability profile. All observations are bounded and policy-labelled.")]
impl ServerHandler for PhaseZeroServer {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        let contract = factorio_agent_contract::canonical_json(&self.contract)
            .unwrap_or_else(|_| "{\"schema_version\":\"unavailable\"}".to_owned());
        rmcp::model::ServerInfo::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            "factorio-agent-mcp",
            "0.1.0",
        ))
        .with_instructions(format!(
            "Phase-0 read-only Factorio capability profile. bridge_contract={contract}"
        ))
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EmptyInput {}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RadiusInput {
    radius: u32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ChartedScanInput {
    center: Position,
    radius: u32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EntityInput {
    entity_id: Option<String>,
    position: Option<Position>,
}

impl EntityInput {
    fn target(self) -> Result<EntityTarget, ContractError> {
        match (self.entity_id, self.position) {
            (Some(entity_id), None) if valid_entity_id(&entity_id) => {
                Ok(EntityTarget::IssuedId(entity_id))
            }
            (None, Some(position)) => {
                validate_position(position)?;
                Ok(EntityTarget::Position(position))
            }
            _ => Err(ContractError::new(
                ErrorCode::InvalidArgument,
                "provide exactly one valid entity_id or position",
            )),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ActionRecordInput {
    sequence: u64,
}

fn validate_radius(radius: u32) -> Result<(), ContractError> {
    if radius == 0 || radius > MAX_HOST_RADIUS {
        return Err(ContractError::new(
            ErrorCode::InvalidArgument,
            "radius is outside the host safety bound",
        ));
    }
    Ok(())
}

fn validate_position(position: Position) -> Result<(), ContractError> {
    if !position.x.is_finite() || !position.y.is_finite() {
        return Err(ContractError::new(
            ErrorCode::InvalidArgument,
            "position must be finite",
        ));
    }
    Ok(())
}

fn valid_entity_id(entity_id: &str) -> bool {
    !entity_id.is_empty()
        && entity_id.len() <= MAX_ENTITY_ID_BYTES
        && entity_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn observation_record(
    capability: Capability,
    result: &factorio_agent_rcon::ObservationResult,
) -> AuditRecord {
    AuditRecord::Observation(ObservationRecord {
        capability,
        game_tick: 0,
        provenance: result.provenance,
        result_count: result.result_count,
        truncated: result.truncated,
    })
}

fn lifecycle_record(name: &str, tick: u64) -> AuditRecord {
    AuditRecord::Lifecycle {
        action_id: ActionId::new(name),
        state: ActionState::Succeeded,
        game_tick: tick,
    }
}

fn internal_mismatch() -> ContractError {
    ContractError::new(
        ErrorCode::Internal,
        "bridge response did not match the approved request",
    )
}

#[allow(clippy::needless_pass_by_value)]
fn error_result(error: ContractError) -> CallToolResult {
    let text = factorio_agent_contract::canonical_json(&error).unwrap_or_else(|_| {
        "{\"code\":\"INTERNAL\",\"detail\":\"serialization failure\"}".to_owned()
    });
    CallToolResult::error(vec![ContentBlock::text(text)])
}

#[allow(clippy::needless_pass_by_value)]
fn map_bridge_error(error: RconError) -> ContractError {
    let code = match error {
        RconError::Bridge { code } => match code {
            BridgeErrorCode::InvalidArgument => ErrorCode::InvalidArgument,
            BridgeErrorCode::ActorUnavailable => ErrorCode::ActorUnavailable,
            BridgeErrorCode::OutOfPolicy => ErrorCode::OutOfPolicy,
            BridgeErrorCode::Uncharted => ErrorCode::Uncharted,
            BridgeErrorCode::NotFound => ErrorCode::NotFound,
            BridgeErrorCode::BridgeUnavailable => ErrorCode::BridgeUnavailable,
            BridgeErrorCode::VersionMismatch => ErrorCode::VersionMismatch,
            BridgeErrorCode::Internal
            | BridgeErrorCode::MalformedResponse
            | BridgeErrorCode::UnknownRemoteError
            | BridgeErrorCode::SerializationFailure
            | BridgeErrorCode::RequestTooLarge
            | BridgeErrorCode::ResponseTooLarge => ErrorCode::Internal,
        },
        RconError::VersionMismatch => ErrorCode::VersionMismatch,
        RconError::Configuration { .. } => ErrorCode::InvalidArgument,
        RconError::Transport { kind } => match kind {
            TransportErrorKind::Authentication
            | TransportErrorKind::Connection
            | TransportErrorKind::Timeout
            | TransportErrorKind::ConnectionLost
            | TransportErrorKind::Framing => ErrorCode::BridgeUnavailable,
        },
    };
    ContractError::new(code, code_string(code))
}

const fn code_string(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::InvalidArgument => "INVALID_ARGUMENT",
        ErrorCode::ActorUnavailable => "ACTOR_UNAVAILABLE",
        ErrorCode::OutOfPolicy => "OUT_OF_POLICY",
        ErrorCode::Uncharted => "UNCHARTED",
        ErrorCode::NotFound => "NOT_FOUND",
        ErrorCode::BridgeUnavailable => "BRIDGE_UNAVAILABLE",
        ErrorCode::VersionMismatch => "VERSION_MISMATCH",
        ErrorCode::Internal => "INTERNAL",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use factorio_agent_contract::{AuditChain, AuditRecord, Provenance};

    #[test]
    fn audit_sink_verifies_continuity_and_refuses_truncation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("audit.jsonl");
        let sink = AuditSink::open(&path).unwrap();
        sink.append(&lifecycle_record("test", 7)).unwrap();
        let reopened = AuditSink::open(&path).unwrap();
        reopened.append(&lifecycle_record("test2", 8)).unwrap();
        let records = load_audit_log(&path).unwrap();
        AuditChain::verify(&records).unwrap();
        fs::write(&path, b"{\"partial\":").unwrap();
        assert!(AuditSink::open(&path).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn audit_sink_refuses_non_regular_log_paths() {
        let Err(error) = AuditSink::open("/dev/full") else {
            panic!("non-regular audit path must be rejected");
        };
        assert_eq!(error.code, ErrorCode::InvalidArgument);
        assert_eq!(error.detail, "audit log must be a regular file");
    }

    #[test]
    fn audit_sink_coordinates_independent_instances() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("audit.jsonl");
        let first = std::sync::Arc::new(AuditSink::open(&path).unwrap());
        let second = std::sync::Arc::new(AuditSink::open(&path).unwrap());
        let ready = std::sync::Arc::new(std::sync::Barrier::new(2));
        let append_from = |sink: std::sync::Arc<AuditSink>,
                           ready: std::sync::Arc<std::sync::Barrier>| {
            std::thread::spawn(move || {
                ready.wait();
                for sequence in 0..16 {
                    sink.append(&lifecycle_record("concurrent", sequence))
                        .unwrap();
                }
            })
        };
        let first_handle = append_from(first, std::sync::Arc::clone(&ready));
        let second_handle = append_from(second, ready);
        first_handle.join().unwrap();
        second_handle.join().unwrap();
        let records = load_audit_log(&path).unwrap();
        assert_eq!(records.len(), 32);
        AuditChain::verify(&records).unwrap();
    }

    #[test]
    fn six_tools_are_the_entire_discovery_surface_and_inputs_are_schemas() {
        let names: Vec<_> = PhaseZeroServer::tool_router()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "get_action_record",
                "get_actor_status",
                "get_capability_contract",
                "get_entity",
                "scan_charted",
                "scan_local",
            ]
        );
        for tool in PhaseZeroServer::tool_router().list_all() {
            assert!(tool.schema_as_json_value().is_object());
            assert_eq!(tool.annotations.unwrap().read_only_hint, Some(true));
        }
    }

    #[test]
    fn validation_rejects_ambiguous_entities_and_unbounded_scans() {
        assert!(validate_radius(0).is_err());
        assert!(validate_radius(MAX_HOST_RADIUS + 1).is_err());
        assert!(
            EntityInput {
                entity_id: Some("x".into()),
                position: Some(Position { x: 1.0, y: 2.0 })
            }
            .target()
            .is_err()
        );
        assert!(
            EntityInput {
                entity_id: Some("bad space".into()),
                position: None
            }
            .target()
            .is_err()
        );
    }

    #[test]
    fn bridge_errors_preserve_the_stable_contract_code() {
        let error = map_bridge_error(RconError::Bridge {
            code: BridgeErrorCode::Uncharted,
        });
        assert_eq!(error.code, ErrorCode::Uncharted);
        assert_eq!(error.detail, "UNCHARTED");
    }

    #[test]
    fn audit_records_have_only_phase_zero_kinds() {
        let record = AuditRecord::Observation(ObservationRecord {
            capability: Capability::ScanLocal,
            game_tick: 1,
            provenance: Provenance::CharacterLocal,
            result_count: 0,
            truncated: false,
        });
        assert!(matches!(record, AuditRecord::Observation(_)));
    }
}
