local actor = require("actor")
local actions = require("actions")
local audit = require("audit")
local read_only = require("read_only")

local protocol = {}

local ALLOWED_REQUESTS = {
  get_actor_status = true,
  get_capability_contract = true,
  scan_local = true,
  scan_charted = true,
  get_entity = true,
  get_action_record = true,
}

local ALLOWED_COMMANDS = {
  walk_to = true,
  stop = true,
  get_action = true,
}

local function rejected(code, detail)
  return { ok = false, error = { code = code, detail = detail } }
end

local function only_fields(request, allowed)
  for field in pairs(request) do
    if not allowed[field] then
      return false
    end
  end
  return true
end

local function request_is_valid(request)
  if request.name == "get_actor_status" or request.name == "get_capability_contract" then
    return only_fields(request, { name = true })
  end
  if request.name == "scan_local" then
    return only_fields(request, { name = true, radius = true }) and request.radius ~= nil
  end
  if request.name == "scan_charted" then
    return only_fields(request, { name = true, center = true, radius = true }) and request.center ~= nil and request.radius ~= nil
  end
  if request.name == "get_entity" then
    return only_fields(request, { name = true, entity_id = true, position = true })
      and ((request.entity_id ~= nil) ~= (request.position ~= nil))
  end
  return only_fields(request, { name = true, sequence = true }) and request.sequence ~= nil
end

local function command_is_valid(request)
  if request.name == "walk_to" then
    return only_fields(request, { name = true, action_id = true, target = true })
      and request.action_id ~= nil and request.target ~= nil
  end
  return only_fields(request, { name = true, action_id = true }) and request.action_id ~= nil
end

local function denied_observation(request, error)
  if ALLOWED_REQUESTS[request.name] and request.name ~= "get_actor_status" and request.name ~= "get_capability_contract" then
    audit.observation(request.name, { center = request.center, radius = request.radius }, nil, "none", 0, false, "rejected:" .. error.code)
  end
end

function protocol.query(request)
  if type(request) ~= "table" or type(request.name) ~= "string" then
    return rejected("INVALID_ARGUMENT", "request must contain a string name")
  end
  if not ALLOWED_REQUESTS[request.name] then
    return rejected("OUT_OF_POLICY", "unknown or future mutation request")
  end
  if not request_is_valid(request) then
    local error = { code = "INVALID_ARGUMENT", detail = "request arguments do not match the read-only capability" }
    denied_observation(request, error)
    return { ok = false, error = error }
  end

  -- Retained records remain inspectable after the actor is unavailable.
  if request.name == "get_action_record" then
    local result, error = read_only.get_action_record(request)
    if not result then
      return { ok = false, error = error }
    end
    return { ok = true, result = result }
  end

  local character, lifecycle_state = actor.resolve_stored()
  if not character then
    return rejected("ACTOR_UNAVAILABLE", lifecycle_state)
  end
  if request.name == "get_actor_status" then
    return { ok = true, result = read_only.actor_status(character, "ready") }
  end
  if request.name == "get_capability_contract" then
    return { ok = true, result = read_only.capability_contract(character) }
  end

  local result, error = read_only[request.name](character, request)
  if not result then
    denied_observation(request, error)
    return { ok = false, error = error }
  end
  return { ok = true, result = result }
end

function protocol.command(request)
  if type(request) ~= "table" or type(request.name) ~= "string" then
    return rejected("INVALID_ARGUMENT", "request must contain a string name")
  end
  if not ALLOWED_COMMANDS[request.name] then
    return rejected("OUT_OF_POLICY", "unknown or future gameplay command")
  end
  if not command_is_valid(request) then
    return rejected("INVALID_ARGUMENT", "request arguments do not match the gameplay command")
  end
  local result, error = actions[request.name](request)
  if not result then
    return { ok = false, error = error }
  end
  return { ok = true, result = result }
end

function protocol.register()
  local config = require("config")
  remote.add_interface(config.BRIDGE_INTERFACE, { query = protocol.query, command = protocol.command })
end

return protocol
