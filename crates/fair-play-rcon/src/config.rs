use std::{fmt, fs, path::PathBuf, time::Duration};

use crate::response::RconError;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TIMEOUT: Duration = Duration::from_secs(60);

/// Bounded time budgets for one RCON read request.
///
/// `factorio-rcon` exposes one deadline for a command round trip, so the adapter uses the
/// shorter of `write` and `read` for that deadline. `connect` is enforced separately.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RconTimeouts {
    pub(crate) connect: Duration,
    pub(crate) write: Duration,
    pub(crate) read: Duration,
}

impl RconTimeouts {
    /// Creates bounded connection and command-round-trip timeouts.
    ///
    /// # Errors
    ///
    /// Returns a configuration error when a timeout is zero or exceeds one minute.
    pub fn new(connect: Duration, write: Duration, read: Duration) -> Result<Self, RconError> {
        for timeout in [connect, write, read] {
            if timeout.is_zero() || timeout > MAX_TIMEOUT {
                return Err(RconError::configuration(
                    "timeout is outside the permitted range",
                ));
            }
        }
        Ok(Self {
            connect,
            write,
            read,
        })
    }
}

impl Default for RconTimeouts {
    fn default() -> Self {
        Self {
            connect: DEFAULT_CONNECT_TIMEOUT,
            write: DEFAULT_WRITE_TIMEOUT,
            read: DEFAULT_READ_TIMEOUT,
        }
    }
}

#[derive(Clone)]
struct Secret(String);

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

/// RCON configuration loaded only from environment or an external secret file.
#[derive(Clone)]
pub struct RconConfig {
    pub(crate) address: String,
    password: Secret,
    pub(crate) timeouts: RconTimeouts,
    pub(crate) expected_bridge_build: String,
}

impl fmt::Debug for RconConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RconConfig")
            .field("address", &self.address)
            .field("password", &self.password)
            .field("timeouts", &self.timeouts)
            .field("expected_bridge_build", &self.expected_bridge_build)
            .finish()
    }
}

impl RconConfig {
    /// Loads connection configuration from `FAIR_PLAY_RCON_*` environment variables.
    ///
    /// Supply exactly one of `FAIR_PLAY_RCON_PASSWORD` or
    /// `FAIR_PLAY_RCON_PASSWORD_FILE`. The password itself is never returned or
    /// included in an error.
    ///
    /// # Errors
    ///
    /// Returns a sanitized configuration error for missing, invalid, or unreadable inputs.
    pub fn from_env() -> Result<Self, RconError> {
        let address = std::env::var("FAIR_PLAY_RCON_ADDRESS")
            .map_err(|_| RconError::configuration("FAIR_PLAY_RCON_ADDRESS is required"))?;
        let password = std::env::var("FAIR_PLAY_RCON_PASSWORD").ok();
        let password_file = std::env::var_os("FAIR_PLAY_RCON_PASSWORD_FILE").map(PathBuf::from);
        let secret = match (password, password_file) {
            (Some(_), Some(_)) => {
                return Err(RconError::configuration("configure one credential source"));
            }
            (Some(password), None) => Secret(validate_secret(&password)?),
            (None, Some(path)) => Secret(read_secret(path)?),
            (None, None) => {
                return Err(RconError::configuration(
                    "an RCON credential source is required",
                ));
            }
        };
        let expected_bridge_build = std::env::var("FAIR_PLAY_RCON_EXPECTED_BRIDGE_BUILD")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_owned());
        Ok(Self {
            address: validate_address(address)?,
            password: secret,
            timeouts: RconTimeouts::default(),
            expected_bridge_build,
        })
    }

    /// Creates configuration using an explicit external password file.
    ///
    /// # Errors
    ///
    /// Returns a sanitized configuration error for an invalid address or unreadable secret file.
    pub fn from_secret_file(
        address: impl Into<String>,
        secret_file: PathBuf,
    ) -> Result<Self, RconError> {
        Ok(Self {
            address: validate_address(address.into())?,
            password: Secret(read_secret(secret_file)?),
            timeouts: RconTimeouts::default(),
            expected_bridge_build: env!("CARGO_PKG_VERSION").to_owned(),
        })
    }

    /// Replaces bounded request timeouts without exposing credentials.
    #[must_use]
    pub fn with_timeouts(mut self, timeouts: RconTimeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    pub(crate) fn password(&self) -> &str {
        &self.password.0
    }
}

fn validate_address(address: String) -> Result<String, RconError> {
    if address.trim().is_empty() || address.len() > 255 || address.contains(['\n', '\r', '\0']) {
        return Err(RconError::configuration("RCON address is invalid"));
    }
    Ok(address)
}

fn read_secret(path: PathBuf) -> Result<String, RconError> {
    let secret = fs::read_to_string(path)
        .map_err(|_| RconError::configuration("unable to read RCON credential file"))?;
    validate_secret(&secret)
}

fn validate_secret(secret: &str) -> Result<String, RconError> {
    let secret = secret.trim_end_matches(['\r', '\n']).to_owned();
    if secret.is_empty() {
        return Err(RconError::configuration("RCON credential is empty"));
    }
    Ok(secret)
}
