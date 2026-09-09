local config = require("config")
local policy = require("scripts.observation_policy")

local read_only = {}

local function inventory_summary(character)
  local inventory = character.get_main_inventory()
  local items = {}
  local total = 0
  local truncated = false
  if inventory then
    for name, count in pairs(inventory.get_contents()) do
      total = total + count
      if #items < config.MAX_INVENTORY_TYPES then
        items[#items + 1] = { name = name, count = count }
      else
        truncated = true
      end
    end
  end
  table.sort(items, function(left, right)
    return left.name < right.name
  end)
  return { total = total, items = items, truncated = truncated }
end

local function observation_result(bounds, provenance)
  return {
    requested_bounds = nil,
    effective_bounds = policy.effective_bounds(bounds),
    provenance = provenance,
    entities = {},
    resources = {},
    tiles = {},
    result_count = 0,
    partial = false,
    truncated = false,
    payload_bytes = 0,
  }
end

local function collect(surface, bounds, provenance, result)
  local area = { bounds.left_top, bounds.right_bottom }
  for _, entity in pairs(surface.find_entities_filtered({ area = area })) do
    if entity.valid then
      if entity.type == "resource" then
        result.resources[#result.resources + 1] = policy.resource_record(entity, provenance)
      else
        result.entities[#result.entities + 1] = policy.entity_record(entity, provenance)
      end
    end
  end
  for _, tile in pairs(surface.get_tiles(area)) do
    result.tiles[#result.tiles + 1] = policy.tile_record(tile, provenance)
  end
end

local function audit_observation(name, requested, result)
  local audit = require("audit")
  audit.observation(name, requested, result.effective_bounds, result.provenance, result.result_count, result.truncated)
end

function read_only.actor_status(character, lifecycle_state)
  local state = storage.factorio_fair_play_bridge
  return {
    state = lifecycle_state,
    actor_id = state.actor.actor_id,
    unit_number = state.actor.unit_number,
    surface = { index = character.surface.index, name = character.surface.name },
    force = { name = character.force.name },
    position = { x = character.position.x, y = character.position.y },
    health = character.health,
    inventory = inventory_summary(character),
    tick = game.tick,
    bridge = {
      build = config.BRIDGE_BUILD,
      lifecycle_schema_version = config.LIFECYCLE_SCHEMA_VERSION,
      policy_version = config.POLICY_VERSION,
    },
  }
end

function read_only.capability_contract(character)
  local status = read_only.actor_status(character, "ready")
  return {
    schema_version = "0.1.0",
    fair_play_policy_version = config.POLICY_VERSION,
    bridge_build = config.BRIDGE_BUILD,
    factorio_build = script.active_mods.base or "unknown",
    enabled_mods = script.active_mods,
    actor_id = status.actor_id,
    force_id = status.force.name,
    profile_name = "phase-zero-read-only",
    scheduling = {
      execution = "read_only",
      observation_tick = "bridge_tick_snapshot",
      request_order = "serialized",
    },
    observation_limits = {
      max_radius = config.MAX_OBSERVATION_RADIUS,
      max_result_count = config.MAX_OBSERVATION_RESULTS,
      max_detail_fields = config.MAX_OBSERVATION_DETAIL_FIELDS,
      max_payload_bytes = config.MAX_OBSERVATION_PAYLOAD_BYTES,
    },
    supported_provenance = { "character_local", "force_charted", "permitted_direct_interaction", "force_statistics" },
    current_game_tick = status.tick,
    capabilities = { "get_capability_contract", "get_actor_status", "scan_local", "scan_charted", "get_entity", "get_action_record" },
  }
end

function read_only.scan_local(character, request)
  local bounds, error = policy.local_bounds(character, request.radius)
  if not bounds then
    return nil, error
  end
  local result = observation_result(bounds, "character_local")
  result.requested_bounds = policy.requested_bounds(request)
  collect(character.surface, bounds, "character_local", result)
  policy.bound_payload(result)
  audit_observation("scan_local", result.requested_bounds, result)
  return result
end

function read_only.scan_charted(character, request)
  local bounds, error = policy.charted_bounds(character, request.center, request.radius)
  if not bounds then
    return nil, error
  end
  local charted, chart_error = policy.require_charted(character.force, character.surface, bounds)
  if not charted then
    return nil, chart_error
  end
  local result = observation_result(bounds, "force_charted")
  result.requested_bounds = policy.requested_bounds(request)
  collect(character.surface, bounds, "force_charted", result)
  policy.bound_payload(result)
  audit_observation("scan_charted", result.requested_bounds, result)
  return result
end

local function coordinate_is_local(character, candidate)
  return math.abs(candidate.x - character.position.x) <= config.MAX_OBSERVATION_RADIUS
    and math.abs(candidate.y - character.position.y) <= config.MAX_OBSERVATION_RADIUS
end

function read_only.get_entity(character, request)
  local entity
  local provenance
  if request.entity_id then
    local unit_number = policy.permitted_entity_id(request.entity_id)
    if not unit_number then
      return nil, { code = "OUT_OF_POLICY", detail = "entity_id was not issued by a permitted bridge observation" }
    end
    entity = game.get_entity_by_unit_number(unit_number)
    provenance = "permitted_direct_interaction"
  else
    local candidate = request.position
    if type(candidate) ~= "table" or type(candidate.x) ~= "number" or type(candidate.y) ~= "number" then
      return nil, { code = "INVALID_ARGUMENT", detail = "get_entity requires a bridge entity_id or numeric position" }
    end
    if coordinate_is_local(character, candidate) then
      provenance = "character_local"
    else
      local bounds = { center = { x = candidate.x, y = candidate.y }, radius = 0, left_top = candidate, right_bottom = candidate }
      local charted, chart_error = policy.require_charted(character.force, character.surface, bounds)
      if not charted then
        return nil, chart_error
      end
      provenance = "force_charted"
    end
    local entities = character.surface.find_entities_filtered({
      area = { { x = candidate.x - 0.25, y = candidate.y - 0.25 }, { x = candidate.x + 0.25, y = candidate.y + 0.25 } },
    })
    entity = entities[1]
  end
  if not (entity and entity.valid) then
    return nil, { code = "NOT_FOUND", detail = "permitted entity does not exist" }
  end
  local result = policy.entity_record(entity, provenance)
  local requested = request.position and { center = request.position, radius = 0 } or { entity_id = request.entity_id }
  audit_observation("get_entity", requested, {
    effective_bounds = { center = result.position, radius = 0 },
    provenance = provenance,
    result_count = 1,
    truncated = false,
  })
  return result
end

function read_only.get_action_record(request)
  if type(request.sequence) ~= "number" or request.sequence % 1 ~= 0 or request.sequence < 0 then
    return nil, { code = "INVALID_ARGUMENT", detail = "sequence must be a non-negative integer" }
  end
  local records = storage.factorio_fair_play_bridge.audit_records
  local record = records[request.sequence + 1]
  if not record then
    return nil, { code = "NOT_FOUND", detail = "action record is not retained" }
  end
  return record
end

return read_only
