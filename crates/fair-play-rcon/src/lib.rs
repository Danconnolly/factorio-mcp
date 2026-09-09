//! Closed-dispatch RCON transport for the Factorio Fair-Play bridge.
//!
//! The public API deliberately has no generic command or Lua evaluation entry
//! point. Named bridge calls will be added after the Phase-0 contract exists.

#![forbid(unsafe_code)]

/// The only bridge interface this crate will target.
pub const BRIDGE_INTERFACE: &str = "factorio_fair_play_bridge";
