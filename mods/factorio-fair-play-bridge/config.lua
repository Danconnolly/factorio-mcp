local config = {}

config.BRIDGE_INTERFACE = "factorio_fair_play_bridge"
config.BRIDGE_BUILD = "0.1.0"
config.LIFECYCLE_SCHEMA_VERSION = 1
config.POLICY_VERSION = "0.1.0"
config.DEFAULT_ACTOR_ID = "alfred"
config.ACTOR_ID_SETTING = "factorio-fair-play-actor-id"
config.MAX_INVENTORY_TYPES = 32
config.MAX_AUDIT_RECORDS = 64

-- These fixed limits are reported by get_capability_contract and enforced by
-- scripts/observation_policy.lua. They are deliberately not client configurable.
config.MAX_OBSERVATION_RADIUS = 32
config.MAX_OBSERVATION_RESULTS = 128
config.MAX_OBSERVATION_DETAIL_FIELDS = 4
config.MAX_OBSERVATION_PAYLOAD_BYTES = 32 * 1024

function config.startup_actor_id()
  local configured = settings.startup[config.ACTOR_ID_SETTING]
  local actor_id = configured and configured.value or config.DEFAULT_ACTOR_ID
  if type(actor_id) ~= "string" or #actor_id > 32 or not actor_id:match("^[a-z][a-z0-9_-]*$") then
    error("factorio-fair-play-bridge: actor ID must match [a-z][a-z0-9_-]{0,31}")
  end
  return actor_id
end

return config
