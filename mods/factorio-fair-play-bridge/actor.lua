local config = require("config")
local audit = require("audit")

local actor = {}

local function bridge_storage()
  return storage.factorio_fair_play_bridge
end

local function marker_for(actor_id)
  return "factorio-fair-play:" .. actor_id .. ":" .. config.POLICY_VERSION
end

local function is_bridge_owned_character(character, record)
  if not (character and character.valid and character.name == "character") then
    return false
  end
  if character.unit_number ~= record.unit_number then
    return false
  end
  if character.surface.index ~= record.surface_index or character.force.name ~= record.force_name then
    return false
  end
  return character.name_tag == record.provenance_marker
end

local function stored_character()
  local state = bridge_storage()
  local record = state.actor
  if type(record) ~= "table" or type(record.unit_number) ~= "number" then
    return nil, "actor record is absent"
  end
  -- Unit-number lookup is the only actor resolution path; human players are never enumerated.
  local character = game.get_entity_by_unit_number(record.unit_number)
  if is_bridge_owned_character(character, record) then
    return character, "ready"
  end
  return nil, "stored actor is unavailable or fails bridge provenance validation"
end

local function create_actor()
  local state = bridge_storage()
  local record = state.actor
  local force = game.forces[record.force_name]
  local surface = game.get_surface(record.surface_name)
  if not force or not surface then
    return nil, "configured force or spawn surface is unavailable"
  end
  local spawn = force.get_spawn_position(surface)
  local position = surface.find_non_colliding_position("character", spawn, 64, 1, true)
  if not position then
    return nil, "no valid non-colliding position at configured force spawn"
  end
  local character = surface.create_entity({
    name = "character",
    position = position,
    force = force,
    raise_built = false,
  })
  if not character or not character.valid or not character.unit_number then
    return nil, "Factorio did not create a virtual character"
  end
  character.name_tag = record.provenance_marker
  record.unit_number = character.unit_number
  record.surface_index = surface.index
  audit.append("actor_created", { actor_id = record.actor_id, unit_number = record.unit_number })
  return character, "ready"
end

local function ensure_record()
  local state = bridge_storage()
  local configured_actor_id = config.startup_actor_id()
  if state.actor and state.actor.actor_id ~= configured_actor_id then
    error("factorio-fair-play-bridge: startup actor ID cannot change after initialization")
  end
  if not state.actor then
    state.actor = {
      actor_id = configured_actor_id,
      unit_number = nil,
      surface_name = "nauvis",
      surface_index = nil,
      force_name = "player",
      provenance_marker = marker_for(configured_actor_id),
      lifecycle_schema_version = config.LIFECYCLE_SCHEMA_VERSION,
      policy_version = config.POLICY_VERSION,
      starting_inventory_policy = "empty",
    }
  end
end

function actor.resolve_stored()
  return stored_character()
end

function actor.ensure()
  ensure_record()
  local character, lifecycle_state = stored_character()
  if character then
    return character, lifecycle_state
  end
  return create_actor()
end

function actor.initialise()
  ensure_record()
  local _, lifecycle_state = actor.ensure()
  audit.append("actor_lifecycle", { state = lifecycle_state })
end

function actor.on_load()
  -- on_load is read-only in Factorio. The next tick resolves the persisted unit
  -- number first and creates only a bridge-marked replacement if it is absent.
end

function actor.validate_after_load()
  if storage.factorio_fair_play_bridge and storage.factorio_fair_play_bridge.actor then
    actor.ensure()
  end
end

return actor
