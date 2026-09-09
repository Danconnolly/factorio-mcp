local config = require("config")

local policy = {}

local function rejected(code, detail)
  return nil, { code = code, detail = detail }
end

local function position(value)
  if type(value) ~= "table" or type(value.x) ~= "number" or type(value.y) ~= "number" then
    return nil
  end
  return { x = value.x, y = value.y }
end

function policy.radius(value)
  if type(value) ~= "number" or value % 1 ~= 0 or value < 0 then
    return rejected("INVALID_ARGUMENT", "radius must be a non-negative integer")
  end
  if value > config.MAX_OBSERVATION_RADIUS then
    return rejected("OUT_OF_POLICY", "requested radius exceeds bridge observation policy")
  end
  return value
end

function policy.local_bounds(character, requested_radius)
  local radius, error = policy.radius(requested_radius)
  if not radius then
    return nil, error
  end
  local center = { x = character.position.x, y = character.position.y }
  return {
    center = center,
    radius = radius,
    left_top = { x = center.x - radius, y = center.y - radius },
    right_bottom = { x = center.x + radius, y = center.y + radius },
  }
end

function policy.charted_bounds(character, requested_center, requested_radius)
  local center = position(requested_center)
  if not center then
    return rejected("INVALID_ARGUMENT", "charted scan requires a numeric center")
  end
  local radius, error = policy.radius(requested_radius)
  if not radius then
    return nil, error
  end
  return {
    center = center,
    radius = radius,
    left_top = { x = center.x - radius, y = center.y - radius },
    right_bottom = { x = center.x + radius, y = center.y + radius },
  }
end

-- Check every chunk before any surface inspection. This prevents a request from
-- using find_entities_filtered, get_tiles, or resource discovery to reveal an
-- uncharted edge of an otherwise charted area.
function policy.require_charted(force, surface, bounds)
  local left = math.floor(bounds.left_top.x / 32)
  local right = math.floor(bounds.right_bottom.x / 32)
  local top = math.floor(bounds.left_top.y / 32)
  local bottom = math.floor(bounds.right_bottom.y / 32)
  for chunk_x = left, right do
    for chunk_y = top, bottom do
      if not force.is_chunk_charted(surface, { x = chunk_x, y = chunk_y }) then
        return rejected("UNCHARTED", "requested bounds include an uncharted force chunk")
      end
    end
  end
  return true
end

function policy.observation_storage()
  local state = storage.factorio_agent_bridge
  state.observation_policy = state.observation_policy or { permitted_entity_ids = {}, order = {} }
  return state.observation_policy
end

function policy.permit_entity(entity)
  if not entity.unit_number then
    return nil
  end
  local observed = policy.observation_storage()
  local id = "bridge-entity:" .. entity.unit_number
  if not observed.permitted_entity_ids[id] then
    observed.permitted_entity_ids[id] = entity.unit_number
    observed.order[#observed.order + 1] = id
    if #observed.order > config.MAX_OBSERVATION_RESULTS * 4 then
      local expired = table.remove(observed.order, 1)
      observed.permitted_entity_ids[expired] = nil
    end
  end
  return id
end

function policy.permitted_entity_id(id)
  if type(id) ~= "string" then
    return nil
  end
  local observed = policy.observation_storage()
  return observed.permitted_entity_ids[id]
end

function policy.entity_record(entity, provenance)
  local detail = {}
  if entity.type then
    detail[#detail + 1] = { name = "type", value = entity.type }
  end
  if entity.health then
    detail[#detail + 1] = { name = "health", value = entity.health }
  end
  if entity.direction then
    detail[#detail + 1] = { name = "direction", value = entity.direction }
  end
  while #detail > config.MAX_OBSERVATION_DETAIL_FIELDS do
    table.remove(detail)
  end
  return {
    entity_id = policy.permit_entity(entity),
    name = entity.name,
    position = { x = entity.position.x, y = entity.position.y },
    provenance = provenance,
    detail = detail,
  }
end

function policy.resource_record(entity, provenance)
  return {
    name = entity.name,
    amount = entity.amount or 0,
    position = { x = entity.position.x, y = entity.position.y },
    provenance = provenance,
  }
end

function policy.tile_record(tile, provenance)
  return {
    name = tile.name,
    position = { x = tile.position.x, y = tile.position.y },
    provenance = provenance,
  }
end

local function by_name_then_position(left, right)
  if left.name ~= right.name then
    return left.name < right.name
  end
  if left.position.x ~= right.position.x then
    return left.position.x < right.position.x
  end
  return left.position.y < right.position.y
end

function policy.sort_results(result)
  table.sort(result.entities, by_name_then_position)
  table.sort(result.resources, by_name_then_position)
  table.sort(result.tiles, by_name_then_position)
end

function policy.bound_payload(result)
  policy.sort_results(result)
  local count = #result.entities + #result.resources + #result.tiles
  while count > config.MAX_OBSERVATION_RESULTS or #helpers.table_to_json(result) > config.MAX_OBSERVATION_PAYLOAD_BYTES do
    if #result.tiles > 0 then
      table.remove(result.tiles)
    elseif #result.resources > 0 then
      table.remove(result.resources)
    elseif #result.entities > 0 then
      table.remove(result.entities)
    else
      break
    end
    result.partial = true
    result.truncated = true
    count = #result.entities + #result.resources + #result.tiles
  end
  result.result_count = count
  result.payload_bytes = #helpers.table_to_json(result)
  return result
end

function policy.requested_bounds(request)
  return { center = request.center, radius = request.radius }
end

function policy.effective_bounds(bounds)
  return { center = bounds.center, radius = bounds.radius }
end

return policy
