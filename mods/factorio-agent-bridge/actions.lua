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

local function inventory_contents(character)
  local inventory = character.get_main_inventory()
  local contents = {}
  if not inventory then return contents end
  -- Factorio 2.x returns an array of { name, quality, count } records rather
  -- than the pre-2.0 name-to-count map. Preserve quality internally so a
  -- mining receipt can neither conflate nor lose item variants.
  for _, entry in pairs(inventory.get_contents()) do
    local quality = entry.quality or "normal"
    contents[entry.name] = contents[entry.name] or {}
    contents[entry.name][quality] = entry.count
  end
  return contents
end

local function inventory_delta(before, character)
  local after = inventory_contents(character)
  local delta = {}
  for name, qualities in pairs(before) do
    for quality, count in pairs(qualities) do
      local after_count = (after[name] and after[name][quality]) or 0
      local difference = after_count - count
      if difference ~= 0 then
        delta[#delta + 1] = { name = name, quality = quality, count = difference }
      end
    end
  end
  for name, qualities in pairs(after) do
    for quality, count in pairs(qualities) do
      local before_count = (before[name] and before[name][quality]) or 0
      if before_count == 0 then
        delta[#delta + 1] = { name = name, quality = quality, count = count }
      end
    end
  end
  table.sort(delta, function(left, right)
    return left.name == right.name and left.quality < right.quality or left.name < right.name
  end)
  return delta
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
    profile_name = "phase-one-mine",
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
  record.inventory_delta = inventory_delta(record.inventory_before or {}, character)
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

local function fingerprint(kind, target, recipe, count)
  if target then
    return kind .. ":" .. string.format("%.17g:%.17g", target.x, target.y)
  end
  if recipe then return kind .. ":" .. recipe .. ":" .. count end
  return kind
end

local function recipe_is_valid(recipe)
  return type(recipe) == "string" and #recipe > 0 and #recipe <= 128
    and recipe:match("^[a-z0-9][a-z0-9_-]*$") ~= nil
end

local function craft_count_is_valid(count)
  return type(count) == "number" and count % 1 == 0 and count > 0 and count <= config.MAX_CRAFT_COUNT
end

local function crafting_queue_snapshot(character)
  local entries = {}
  for _, entry in pairs(character.crafting_queue) do
    entries[#entries + 1] = { index = entry.index, recipe = entry.recipe, count = entry.count, prerequisite = entry.prerequisite }
  end
  table.sort(entries, function(left, right) return left.index < right.index end)
  return entries
end

local function crafting_queue_fingerprint(character)
  local parts = { tostring(character.crafting_queue_size), string.format("%.9g", character.crafting_queue_progress) }
  for _, entry in pairs(crafting_queue_snapshot(character)) do
    parts[#parts + 1] = entry.index .. ":" .. entry.recipe .. ":" .. entry.count .. ":" .. tostring(entry.prerequisite)
  end
  return table.concat(parts, "|")
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

local function target_descriptor(entity)
  return {
    name = entity.name,
    type = entity.type,
    unit_number = entity.unit_number,
    position = { x = entity.position.x, y = entity.position.y },
  }
end

local function resolve_mine_target(character, target)
  if squared_distance(character.position, target) > character.resource_reach_distance * character.resource_reach_distance then
    return nil, { code = "OUT_OF_POLICY", detail = "target exceeds the actor resource reach distance" }
  end
  character.update_selected_entity(target)
  local entity = character.selected
  if not (entity and entity.valid) then
    return nil, { code = "NOT_FOUND", detail = "no mineable entity exists at the target position" }
  end
  if squared_distance(entity.position, target) > config.MINE_TARGET_TOLERANCE * config.MINE_TARGET_TOLERANCE then
    return nil, { code = "NOT_FOUND", detail = "selected entity does not match the requested target position" }
  end
  if not entity.minable then
    return nil, { code = "OUT_OF_POLICY", detail = "target is not currently mineable" }
  end
  return entity
end

local function clear_character_input(character, record)
  if record.kind == "walk_to" then
    character.walking_state = { walking = false, direction = record.last_direction or defines.direction.north }
  elseif record.kind == "mine" then
    character.mining_state = { mining = false }
  elseif record.kind == "craft" then
    -- Admission requires an empty queue, so all current queue work is this action's.
    local attempts = 0
    while character.crafting_queue_size > 0 and attempts < config.MAX_CRAFT_COUNT * 8 do
      local entry = character.crafting_queue[1]
      if not entry then break end
      character.cancel_crafting({ index = entry.index, count = entry.count })
      attempts = attempts + 1
    end
  end
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
  record.inventory_before = inventory_contents(character)
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
    clear_character_input(character, active)
    if active.kind == "craft" then
      active.queue_after = crafting_queue_snapshot(character)
      active.cancelled_count = active.started_count - active.completed_count
    end
    finalise(character, active, "cancelled", "STOP_REQUESTED", "cancelled by a serialized stop request")
    state.active_action_id = nil
    stop_record.result = { code = "STOPPED", reason = "active gameplay input was cleared" }
  end
  state.actions_by_id[request.action_id] = stop_record
  audit.mutation(stop_record)
  return stop_record
end

function actions.mine(request)
  if type(request) ~= "table" or not action_id_is_valid(request.action_id) or not target_is_valid(request.target) then
    return nil, { code = "INVALID_ARGUMENT", detail = "mine requires a valid action_id and finite target" }
  end
  local request_fingerprint = fingerprint("mine", request.target)
  local existing, conflict = existing_or_conflict(request.action_id, request_fingerprint)
  if existing then return existing end
  if conflict then return nil, conflict end

  local character, lifecycle_state = actor.resolve_stored()
  if not character then
    return nil, { code = "ACTOR_UNAVAILABLE", detail = lifecycle_state }
  end
  local target, target_error = resolve_mine_target(character, request.target)
  if not target then return nil, target_error end
  local state = bridge_storage()
  if state.active_action_id then
    return nil, { code = "ACTION_IN_PROGRESS", detail = "another gameplay action is active" }
  end

  local record = receipt(character, request.action_id, "mine", "accepted", { code = "ACCEPTED", reason = "scheduled for next game tick" })
  record.target = { x = request.target.x, y = request.target.y }
  record.target_entity = target_descriptor(target)
  record.target_amount_before = target.type == "resource" and target.amount or nil
  record.request_fingerprint = request_fingerprint
  record.inventory_before = inventory_contents(character)
  record.last_mining_progress = 0
  record.stalled_ticks = 0
  state.actions_by_id[request.action_id] = record
  state.active_action_id = request.action_id
  audit.mutation(record)
  return record
end

function actions.craft(request)
  if type(request) ~= "table" or not action_id_is_valid(request.action_id)
    or not recipe_is_valid(request.recipe) or not craft_count_is_valid(request.count) then
    return nil, { code = "INVALID_ARGUMENT", detail = "craft requires a valid action_id, recipe, and bounded positive count" }
  end
  local request_fingerprint = fingerprint("craft", nil, request.recipe, request.count)
  local existing, conflict = existing_or_conflict(request.action_id, request_fingerprint)
  if existing then return existing end
  if conflict then return nil, conflict end

  local character, lifecycle_state = actor.resolve_stored()
  if not character then return nil, { code = "ACTOR_UNAVAILABLE", detail = lifecycle_state } end
  -- Factorio's virtual character in the current headless API does not expose
  -- the native hand-crafting queue that a LuaPlayer owns. Fail closed rather
  -- than claiming queued ordinary crafting or touching inventory ourselves.
  if type(character.begin_crafting) ~= "function" or type(character.crafting_queue) ~= "table"
    or type(character.crafting_queue_size) ~= "number" then
    return nil, { code = "OUT_OF_POLICY", detail = "the configured virtual actor has no native hand-crafting queue; use a connected player actor" }
  end
  local recipe = character.force.recipes[request.recipe]
  if not (recipe and recipe.valid) then
    return nil, { code = "NOT_FOUND", detail = "recipe does not exist for the actor force" }
  end
  if not recipe.enabled then
    return nil, { code = "OUT_OF_POLICY", detail = "recipe is not enabled for the actor force" }
  end
  if character.crafting_queue_size ~= 0 then
    return nil, { code = "ACTION_IN_PROGRESS", detail = "native crafting queue must be empty before a bridge craft action" }
  end
  local state = bridge_storage()
  if state.active_action_id then
    return nil, { code = "ACTION_IN_PROGRESS", detail = "another gameplay action is active" }
  end

  local record = receipt(character, request.action_id, "craft", "accepted", { code = "ACCEPTED", reason = "native hand-crafting is scheduled for next game tick" })
  record.recipe = request.recipe
  record.requested_count = request.count
  record.started_count = 0
  record.completed_count = 0
  record.queue_before = crafting_queue_snapshot(character)
  record.request_fingerprint = request_fingerprint
  record.inventory_before = inventory_contents(character)
  state.actions_by_id[request.action_id] = record
  state.active_action_id = request.action_id
  audit.mutation(record)
  return record
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
    record.result = { code = "RUNNING", reason = "ordinary character input is applied every game tick" }
    record.start_tick = game.tick
    record.start_position = current.position
    record.start_health = current.health
  end
  if record.kind == "mine" then
    local target, target_error = resolve_mine_target(character, record.target)
    if not target then
      clear_character_input(character, record)
      local delta = inventory_delta(record.inventory_before, character)
      if next(delta) then
        finalise(character, record, "succeeded", "TARGET_DEPLETED", "target became unavailable after ordinary mining output")
      else
        finalise(character, record, "failed", target_error.code, target_error.detail)
      end
      state.active_action_id = nil
      return
    end
    character.mining_state = { mining = true, position = record.target }
    local delta = inventory_delta(record.inventory_before, character)
    if next(delta) then
      clear_character_input(character, record)
      record.target_amount_after = target.type == "resource" and target.amount or nil
      finalise(character, record, "succeeded", "MINED", "ordinary character mining produced one inventory delta")
      state.active_action_id = nil
      return
    end
    local progress = character.character_mining_progress
    if progress <= record.last_mining_progress then
      record.stalled_ticks = record.stalled_ticks + 1
    else
      record.stalled_ticks = 0
      record.last_mining_progress = progress
    end
    if record.stalled_ticks >= config.MAX_STALLED_MINE_TICKS then
      clear_character_input(character, record)
      local inventory = character.get_main_inventory()
      local code = inventory and inventory.is_full() and "INVENTORY_FULL" or "BLOCKED"
      finalise(character, record, "failed", code, "ordinary mining made no measurable progress")
      state.active_action_id = nil
    end
    return
  end
  if record.kind == "craft" then
    if not record.craft_started then
      -- This is the only crafting call. The native queue controls ingredients,
      -- prerequisites, speed, timing, capacity, and output without simulation.
      record.started_count = character.begin_crafting({ recipe = record.recipe, count = record.requested_count })
      record.craft_started = true
      record.queue_started = crafting_queue_snapshot(character)
      record.last_craft_queue_fingerprint = crafting_queue_fingerprint(character)
      record.stalled_ticks = 0
      if record.started_count == 0 then
        finalise(character, record, "failed", "NOT_CRAFTABLE", "native hand-crafting could not start the requested recipe")
        state.active_action_id = nil
      end
      return
    end
    if character.crafting_queue_size == 0 then
      record.completed_count = record.started_count
      record.queue_after = {}
      finalise(character, record, "succeeded", "CRAFTED", "native hand-crafting queue completed")
      state.active_action_id = nil
      return
    end
    local queue_fingerprint = crafting_queue_fingerprint(character)
    if queue_fingerprint == record.last_craft_queue_fingerprint then
      record.stalled_ticks = record.stalled_ticks + 1
    else
      record.stalled_ticks = 0
      record.last_craft_queue_fingerprint = queue_fingerprint
    end
    if record.stalled_ticks >= config.MAX_STALLED_CRAFT_TICKS then
      record.queue_after = crafting_queue_snapshot(character)
      finalise(character, record, "failed", "BLOCKED", "native hand-crafting queue made no measurable progress")
      state.active_action_id = nil
    end
    return
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
