local actor = require("actor")
local audit = require("audit")
local config = require("config")

local actions = {}

local TERMINAL = {
  succeeded = true,
  rejected = true,
  cancelled = true,
  failed = true,
}

local function bridge_storage()
  return storage.factorio_agent_bridge
end

local function squared_distance(left, right)
  local x = left.x - right.x
  local y = left.y - right.y
  return x * x + y * y
end

local function snapshot(character)
  return {
    position = { x = character.position.x, y = character.position.y },
    health = character.health,
  }
end

local function context(character)
  local state = bridge_storage()
  return {
    schema_version = config.SCHEMA_VERSION,
    fair_play_policy_version = config.POLICY_VERSION,
    bridge_build = config.BRIDGE_BUILD,
    factorio_build = script.active_mods.base or "unknown",
    actor_id = state.actor.actor_id,
    actor_unit_number = state.actor.unit_number,
    force_id = character.force.name,
    surface = { index = character.surface.index, name = character.surface.name },
    profile_name = "phase-one-walk-stop",
  }
end

local function receipt(character, action_id, kind, state, result)
  local state_storage = bridge_storage()
  state_storage.next_action_sequence = (state_storage.next_action_sequence or 0) + 1
  local current = snapshot(character)
  local record = context(character)
  record.action_id = action_id
  record.action_sequence = state_storage.next_action_sequence
  record.kind = kind
  record.state = state
  record.result = result
  record.accepted_tick = game.tick
  record.start_tick = nil
  record.end_tick = nil
  record.start_position = current.position
  record.end_position = current.position
  record.start_health = current.health
  record.end_health = current.health
  -- Movement must neither create nor consume items; an explicit empty delta is
  -- retained so consumers never mistake this for an unmeasured field.
  record.inventory_delta = {}
  record.affected_entity_ids = {}
  return record
end

local function finalise(character, record, state, code, reason)
  local current = snapshot(character)
  record.state = state
  record.result = { code = code, reason = reason }
  record.end_tick = game.tick
  record.end_position = current.position
  record.end_health = current.health
  audit.mutation(record)
end

local function action_id_is_valid(action_id)
  return type(action_id) == "string"
    and #action_id > 0
    and #action_id <= config.MAX_ACTION_ID_BYTES
    and action_id:match("^[A-Za-z0-9][A-Za-z0-9_.:-]*$") ~= nil
end

local function target_is_valid(target)
  return type(target) == "table"
    and type(target.x) == "number"
    and type(target.y) == "number"
    and target.x == target.x
    and target.y == target.y
    and target.x ~= math.huge
    and target.x ~= -math.huge
    and target.y ~= math.huge
    and target.y ~= -math.huge
end

local function fingerprint(kind, target)
  if target then
    return kind .. ":" .. string.format("%.17g:%.17g", target.x, target.y)
  end
  return kind
end

local function existing_or_conflict(action_id, request_fingerprint)
  local record = bridge_storage().actions_by_id[action_id]
  if not record then
    return nil
  end
  if record.request_fingerprint ~= request_fingerprint then
    return false, { code = "ACTION_ID_CONFLICT", detail = "action_id was already used for another request" }
  end
  return record
end

local function direction_toward(from, target)
  local x = target.x - from.x
  local y = target.y - from.y
  local horizontal = math.abs(x)
  local vertical = math.abs(y)
  if horizontal > vertical * 2 then
    return x > 0 and defines.direction.east or defines.direction.west
  end
  if vertical > horizontal * 2 then
    return y > 0 and defines.direction.south or defines.direction.north
  end
  if x > 0 and y > 0 then return defines.direction.southeast end
  if x > 0 and y < 0 then return defines.direction.northeast end
  if x < 0 and y > 0 then return defines.direction.southwest end
  return defines.direction.northwest
end

function actions.initialise()
  local state = bridge_storage()
  state.actions_by_id = state.actions_by_id or {}
  state.active_action_id = state.active_action_id or nil
  state.next_action_sequence = state.next_action_sequence or 0
end

function actions.walk_to(request)
  if type(request) ~= "table" or not action_id_is_valid(request.action_id) or not target_is_valid(request.target) then
    return nil, { code = "INVALID_ARGUMENT", detail = "walk_to requires a valid action_id and finite target" }
  end
  local request_fingerprint = fingerprint("walk_to", request.target)
  local existing, conflict = existing_or_conflict(request.action_id, request_fingerprint)
  if existing then return existing end
  if conflict then return nil, conflict end

  local character, lifecycle_state = actor.resolve_stored()
  if not character then
    return nil, { code = "ACTOR_UNAVAILABLE", detail = lifecycle_state }
  end
  if squared_distance(character.position, request.target) > config.MAX_WALK_DISTANCE * config.MAX_WALK_DISTANCE then
    return nil, { code = "OUT_OF_POLICY", detail = "target exceeds the fixed walk distance policy" }
  end
  local state = bridge_storage()
  if state.active_action_id then
    return nil, { code = "ACTION_IN_PROGRESS", detail = "another gameplay action is active" }
  end

  local record = receipt(character, request.action_id, "walk_to", "accepted", { code = "ACCEPTED", reason = "scheduled for next game tick" })
  record.target = { x = request.target.x, y = request.target.y }
  record.request_fingerprint = request_fingerprint
  record.last_position = record.start_position
  record.stalled_ticks = 0
  state.actions_by_id[request.action_id] = record
  state.active_action_id = request.action_id
  audit.mutation(record)
  return record
end

function actions.stop(request)
  if type(request) ~= "table" or not action_id_is_valid(request.action_id) then
    return nil, { code = "INVALID_ARGUMENT", detail = "stop requires a valid action_id" }
  end
  local request_fingerprint = fingerprint("stop", nil)
  local existing, conflict = existing_or_conflict(request.action_id, request_fingerprint)
  if existing then return existing end
  if conflict then return nil, conflict end

  local character, lifecycle_state = actor.resolve_stored()
  if not character then
    return nil, { code = "ACTOR_UNAVAILABLE", detail = lifecycle_state }
  end
  local state = bridge_storage()
  local stop_record = receipt(character, request.action_id, "stop", "succeeded", { code = "NO_ACTIVE_ACTION", reason = "no gameplay action was running" })
  stop_record.request_fingerprint = request_fingerprint
  if state.active_action_id then
    local active = state.actions_by_id[state.active_action_id]
    character.walking_state = { walking = false, direction = active.last_direction or defines.direction.north }
    finalise(character, active, "cancelled", "STOP_REQUESTED", "cancelled by a serialized stop request")
    state.active_action_id = nil
    stop_record.result = { code = "STOPPED", reason = "active walk input was cleared" }
  end
  state.actions_by_id[request.action_id] = stop_record
  audit.mutation(stop_record)
  return stop_record
end

function actions.get_action(request)
  if type(request) ~= "table" or not action_id_is_valid(request.action_id) then
    return nil, { code = "INVALID_ARGUMENT", detail = "get_action requires a valid action_id" }
  end
  local record = bridge_storage().actions_by_id[request.action_id]
  if not record then
    return nil, { code = "NOT_FOUND", detail = "action_id is not retained" }
  end
  return record
end

function actions.tick()
  local state = bridge_storage()
  local action_id = state.active_action_id
  if not action_id then return end
  local record = state.actions_by_id[action_id]
  if not record or TERMINAL[record.state] then
    state.active_action_id = nil
    return
  end
  local character, lifecycle_state = actor.resolve_stored()
  if not character then
    -- An unavailable actor cannot be controlled; retain the last known snapshot.
    record.state = "failed"
    record.result = { code = "ACTOR_UNAVAILABLE", reason = lifecycle_state }
    record.end_tick = game.tick
    state.active_action_id = nil
    audit.mutation(record)
    return
  end
  if record.state == "accepted" then
    local current = snapshot(character)
    record.state = "running"
    record.result = { code = "RUNNING", reason = "ordinary walking input is applied every game tick" }
    record.start_tick = game.tick
    record.start_position = current.position
    record.start_health = current.health
  end
  if squared_distance(character.position, record.target) <= config.WALK_TARGET_TOLERANCE * config.WALK_TARGET_TOLERANCE then
    character.walking_state = { walking = false, direction = record.last_direction or defines.direction.north }
    finalise(character, record, "succeeded", "TARGET_REACHED", "actor reached the policy target tolerance")
    state.active_action_id = nil
    return
  end
  local direction = direction_toward(character.position, record.target)
  character.walking_state = { walking = true, direction = direction }
  record.last_direction = direction
  local current = snapshot(character)
  if squared_distance(current.position, record.last_position) <= config.WALK_PROGRESS_EPSILON * config.WALK_PROGRESS_EPSILON then
    record.stalled_ticks = record.stalled_ticks + 1
  else
    record.stalled_ticks = 0
    record.last_position = current.position
  end
  if record.stalled_ticks >= config.MAX_STALLED_WALK_TICKS then
    character.walking_state = { walking = false, direction = direction }
    finalise(character, record, "failed", "BLOCKED", "actor made no measurable progress while walking input was requested")
    state.active_action_id = nil
  end
end

return actions
