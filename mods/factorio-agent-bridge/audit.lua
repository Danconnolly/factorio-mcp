local config = require("config")

local audit = {}

local function bridge_storage()
  return storage.factorio_agent_bridge
end

function audit.append(kind, detail)
  local state = bridge_storage()
  local records = state.audit_records
  records[#records + 1] = {
    kind = kind,
    detail = detail,
    tick = game.tick,
  }
  if #records > config.MAX_AUDIT_RECORDS then
    table.remove(records, 1)
  end
end

-- Observation audit records intentionally contain only policy metadata. They do
-- not retain observed world records, chat, or player material.
function audit.observation(capability, requested_bounds, effective_bounds, provenance, result_count, truncated, decision)
  audit.append("observation", {
    capability = capability,
    decision = decision or "allowed",
    requested_bounds = requested_bounds,
    effective_bounds = effective_bounds,
    provenance = provenance,
    result_count = result_count or 0,
    truncated = truncated or false,
  })
end

-- Receipts are persisted by actions.lua for idempotency. This bounded index is
-- audit-only and must never be used as the source of an action result.
function audit.mutation(receipt)
  audit.append("mutation", {
    action_id = receipt.action_id,
    action_sequence = receipt.action_sequence,
    kind = receipt.kind,
    state = receipt.state,
    result = receipt.result,
  })
end

return audit
