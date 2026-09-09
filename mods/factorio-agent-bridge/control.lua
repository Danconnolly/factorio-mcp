-- Factorio Agent Bridge production mod.
-- Phase 0 exposes a fixed read-only remote query, never commands or raw Lua.

local config = require("config")
local actor = require("actor")
local protocol = require("protocol")

protocol.register()

local function initialise_storage()
  storage.factorio_agent_bridge = storage.factorio_agent_bridge or {
    bridge_interface = config.BRIDGE_INTERFACE,
    lifecycle_schema_version = config.LIFECYCLE_SCHEMA_VERSION,
    policy_version = config.POLICY_VERSION,
    audit_records = {},
  }
  local state = storage.factorio_agent_bridge
  state.audit_records = state.audit_records or {}
  state.bridge_interface = config.BRIDGE_INTERFACE
  state.lifecycle_schema_version = config.LIFECYCLE_SCHEMA_VERSION
  state.policy_version = config.POLICY_VERSION
  actor.initialise()
end

script.on_init(function()
  initialise_storage()
end)

script.on_configuration_changed(function()
  initialise_storage()
end)

script.on_load(function()
  actor.on_load()
end)

script.on_nth_tick(1, actor.validate_after_load)
