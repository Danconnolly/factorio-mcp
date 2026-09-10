local config = {}

config.BRIDGE_INTERFACE = "factorio_agent_bridge"
config.BRIDGE_BUILD = "0.4.0"
config.LIFECYCLE_SCHEMA_VERSION = 1
config.SCHEMA_VERSION = "0.4.0"
config.POLICY_VERSION = "0.4.0"
config.DEFAULT_ACTOR_ID = "alfred"
config.ACTOR_ID_SETTING = "factorio-agent-bridge-actor-id"
config.MAX_INVENTORY_TYPES = 32
config.MAX_AUDIT_RECORDS = 64
config.MAX_ACTION_ID_BYTES = 128
config.MAX_WALK_DISTANCE = 128
config.WALK_TARGET_TOLERANCE = 0.2
config.WALK_PROGRESS_EPSILON = 0.001
config.MAX_STALLED_WALK_TICKS = 120
config.MAX_STALLED_MINE_TICKS = 120
config.MINE_TARGET_TOLERANCE = 0.25
-- Crafting starts only from an empty native queue, so a stop request can cancel
-- only work that this bridge action caused without touching external work.
config.MAX_CRAFT_COUNT = 100
config.MAX_STALLED_CRAFT_TICKS = 120

-- These fixed limits are reported by get_capability_contract and enforced by
-- scripts/observation_policy.lua. They are deliberately not client configurable.
config.MAX_OBSERVATION_RADIUS = 32
config.MAX_OBSERVATION_RESULTS = 128
config.MAX_OBSERVATION_DETAIL_FIELDS = 4
-- Factorio's RCON console response framing is materially smaller than the host
-- adapter's 64 KiB guard. Keep the bridge result below 3 KiB so the complete
-- JSON envelope remains transportable on the real game server.
config.MAX_OBSERVATION_PAYLOAD_BYTES = 3 * 1024

function config.startup_actor_id()
  local configured = settings.startup[config.ACTOR_ID_SETTING]
  local actor_id = configured and configured.value or config.DEFAULT_ACTOR_ID
  if type(actor_id) ~= "string" or #actor_id > 32 or not actor_id:match("^[a-z][a-z0-9_-]*$") then
    error("factorio-agent-bridge: actor ID must match [a-z][a-z0-9_-]{0,31}")
  end
  return actor_id
end

return config
