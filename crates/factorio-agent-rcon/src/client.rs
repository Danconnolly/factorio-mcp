use std::sync::Arc;

use factorio_rcon::{RconClient, RconError as RawRconError};
use tokio::{sync::Mutex, time::timeout};

use crate::{
    command::BridgeCommand,
    config::RconConfig,
    response::{
        BridgeErrorCode, BridgeResponse, RconError, TransportErrorKind, decode_action,
        decode_action_record, decode_actor_status, decode_contract, decode_entity,
        decode_observation,
    },
};

const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const READ_ATTEMPTS: usize = 2;

/// Serialized fixed-dispatch client for the production bridge interface.
///
/// The underlying RCON client and its arbitrary-command method are deliberately
/// private. Every public request is one `BridgeCommand` variant.
pub struct RconAdapter {
    config: RconConfig,
    /// Serializes the complete request lifecycle, including retries and reconnects.
    request_lock: Mutex<()>,
    client: Arc<Mutex<RconClient>>,
}

impl RconAdapter {
    /// Connects, authenticates, and verifies bridge metadata before exposing the adapter.
    ///
    /// # Errors
    ///
    /// Returns a sanitized transport, bridge, or version-mismatch error.
    pub async fn connect(config: RconConfig) -> Result<Self, RconError> {
        let client = connect_raw(&config).await?;
        let adapter = Self {
            config,
            request_lock: Mutex::new(()),
            client: Arc::new(Mutex::new(client)),
        };
        let BridgeResponse::CapabilityContract(contract) =
            adapter.call(BridgeCommand::GetCapabilityContract).await?
        else {
            return Err(RconError::VersionMismatch);
        };
        if !contract.schema_version_matches_build()
            || contract.bridge_build != adapter.config.expected_bridge_build
        {
            return Err(RconError::VersionMismatch);
        }
        Ok(adapter)
    }

    /// # Errors
    ///
    /// Returns a sanitized transport or bridge error. Mutation calls are issued
    /// exactly once: a timeout or connection loss is ambiguous and callers must
    /// recover through `GetAction` using the same action ID.
    pub async fn call(&self, command: BridgeCommand) -> Result<BridgeResponse, RconError> {
        let _request_guard = self.request_lock.lock().await;
        let lua = command.lua_command()?;
        let attempts = if command.is_mutation() {
            1
        } else {
            READ_ATTEMPTS
        };
        for attempt in 0..attempts {
            match self.execute_read(&lua).await {
                Ok(payload) => return decode_response(&command, &payload),
                Err(error) if attempt + 1 < attempts && retryable(&error) => {
                    self.reconnect().await?;
                }
                Err(error) => return Err(error),
            }
        }
        Err(RconError::Transport {
            kind: TransportErrorKind::Connection,
        })
    }

    async fn execute_read(&self, command: &str) -> Result<String, RconError> {
        let mut client = self.client.lock().await;
        let response = client
            .execute_with_timeout(
                command,
                self.config.timeouts.write.min(self.config.timeouts.read),
            )
            .await
            .map_err(|error| map_raw_error(&error))?;
        if response.len() > MAX_RESPONSE_BYTES {
            return Err(RconError::Bridge {
                code: BridgeErrorCode::ResponseTooLarge,
            });
        }
        Ok(response)
    }

    async fn reconnect(&self) -> Result<(), RconError> {
        let replacement = connect_raw(&self.config).await?;
        let mut client = self.client.lock().await;
        *client = replacement;
        Ok(())
    }
}

async fn connect_raw(config: &RconConfig) -> Result<RconClient, RconError> {
    timeout(
        config.timeouts.connect,
        RconClient::connect(&config.address, config.password()),
    )
    .await
    .map_err(|_| RconError::Transport {
        kind: TransportErrorKind::Timeout,
    })?
    .map_err(|error| map_raw_error(&error))
}

fn decode_response(command: &BridgeCommand, payload: &str) -> Result<BridgeResponse, RconError> {
    match command {
        BridgeCommand::GetCapabilityContract => {
            decode_contract(payload).map(BridgeResponse::CapabilityContract)
        }
        BridgeCommand::GetActorStatus => {
            decode_actor_status(payload).map(BridgeResponse::ActorStatus)
        }
        BridgeCommand::ScanLocal { .. } => {
            decode_observation(payload).map(BridgeResponse::ScanLocal)
        }
        BridgeCommand::ScanCharted { .. } => {
            decode_observation(payload).map(BridgeResponse::ScanCharted)
        }
        BridgeCommand::GetEntity { .. } => decode_entity(payload).map(BridgeResponse::Entity),
        BridgeCommand::GetActionRecord { .. } => {
            decode_action_record(payload).map(BridgeResponse::ActionRecord)
        }
        BridgeCommand::WalkTo { .. }
        | BridgeCommand::Stop { .. }
        | BridgeCommand::GetAction { .. } => decode_action(payload).map(BridgeResponse::Action),
    }
}

fn retryable(error: &RconError) -> bool {
    matches!(
        error,
        RconError::Transport {
            kind: TransportErrorKind::Connection
                | TransportErrorKind::ConnectionLost
                | TransportErrorKind::Timeout
        }
    )
}

fn map_raw_error(error: &RawRconError) -> RconError {
    let kind = match error {
        RawRconError::ConnectionFailed(_) => TransportErrorKind::Connection,
        RawRconError::AuthFailed => TransportErrorKind::Authentication,
        RawRconError::Timeout(_) => TransportErrorKind::Timeout,
        RawRconError::ConnectionLost(_) => TransportErrorKind::ConnectionLost,
        RawRconError::ProtocolError(_) | RawRconError::InvalidPacket(_) | RawRconError::Io(_) => {
            TransportErrorKind::Framing
        }
    };
    RconError::Transport { kind }
}
