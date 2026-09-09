# Factorio Fair-Play MCP Implementation Plan

## Decision

Build a new strict, headless bridge rather than adopting or forking an existing controller as the product.

The project targets a persistent virtual character in a headless Factorio 2.x server. This is the only route that meets the zero-connected-human-client requirement. It must earn its fair-play claim through differential tests against a real human-controlled player; it must not claim native `LuaPlayer` equivalence by assertion.

If the required movement, mining, crafting, building, death, inventory, or relevant mod-event behaviour cannot be demonstrated, the project must pivot to a permanently connected dedicated Factorio client. The requirements must not be weakened to fit a convenient Lua API.

## Reuse boundaries

- Use `PlasmaChroma/FactorioMCP` as architectural reference for bounded observation, typed capability policy, durable sessions and audit receipts.
- Use `ai-player-v3` only as reference material for virtual-character lifecycle. Do not inherit its gameplay action code or MCP surface.
- Use FLE only in disposable evaluation/test environments; never attach its stock MCP to a valued human save.
- Do not expose raw RCON/Lua through the gameplay MCP.

## Target architecture

### Factorio 2.x bridge mod

- Own one configured actor identity, initially `alfred`.
- Create or recover the actor once and persist its identity, unit number and policy version in `storage`.
- Expose only a narrow named remote interface; it is not a general Lua execution service.
- Run gameplay commands through tick-driven action state machines.
- Enforce observation and interaction policy inside the mod, not just in the MCP server.
- Emit structured mutation receipts to an append-only audit sink.

### MCP server

- Offers only typed gameplay tools from the approved capability manifest.
- Validates all arguments before forwarding commands to the bridge mod.
- Returns the bridge receipt unchanged or wrapped without losing required fields.
- Contains no generic RCON console, raw Lua, reset, inventory-set, speed, teleport or broad map-inspection capability.

### Operator/admin service

- Is a separate endpoint and credential domain from gameplay MCP.
- Is disabled by default.
- Is limited to save backup/restore, test-world provisioning and operational recovery.
- Emits separately marked audit records.

### Test environment

- Uses a dedicated disposable Factorio server and copied saves.
- Uses a real client-controlled player for differential tests.
- Runs human-isolation tests with both actors present.
- Never uses the valued shared save until the release gates are satisfied.

## Delivery phases

## Phase 0 — feasibility and read-only foundation

Deliver:

1. Repository structure, Factorio 2.x mod skeleton and MCP server skeleton.
2. Configured actor identity plus create/recover lifecycle.
3. Zero-client persistence across MCP reconnect, save/reload and server restart.
4. Bounded, chart-aware read-only observation.
5. Capability manifest containing only read operations.
6. Structured, append-only audit-receipt format.
7. Differential-test harness with a dedicated test client.

Required gate:

Run initial probes comparing virtual actor and human player behaviour for movement, mining, hand crafting, placement, inventory capacity and death/respawn. Record elapsed ticks, positions, inventory deltas, results and observed events.

Decision:

- Continue only if the virtual actor can plausibly meet the requirements through tested mechanics.
- Pivot to a dedicated connected client if material behaviour or required events cannot match.

## Phase 1 — strict primitive gameplay actions

Add one primitive at a time, with a receipt and conservation test before proceeding:

1. Character status, bounded scans, entity/prototype/recipe/research queries.
2. `walk_to` and `stop`.
3. `mine`.
4. `craft`.
5. `place_entity` and `rotate`.
6. `insert`, `extract`, `set_recipe` and `pickup`.
7. `select_research` and measured `wait`.

Every action must report actor ID, action ID, result or rejection reason, start/end tick, start/end position, health, inventory delta, affected entity IDs and fair-play policy version.

Do not add blueprint execution, batch construction, production macros or other convenience operations during this phase.

## Phase 2 — executable acceptance suite

Make the verification requirements CI gates:

- Capability-manifest and static scans prove prohibited gameplay paths are absent.
- Tests cover item conservation for every construction and transfer path.
- Tests demonstrate zero-client operation and save/restart persistence.
- Tests prove connected humans are neither selected nor modified.
- Differential tests cover reach, collision, timing, inventory and relevant events.
- Death/respawn tests detect duplication and consequence bypass.
- An unattended end-to-end test builds a factory and measures at least 12 iron gears during each of two consecutive 60-game-second windows.

The end-to-end test must publish the measured production result, audit log and save identifier/checksum as build artifacts.

## Phase 3 — controlled live-world pilot

Only after Phase 2 passes:

1. Back up the target save and test rollback.
2. Install the bridge with the agent inventory explicitly empty by default.
3. Begin with observation-only operation.
4. Assign the agent a geographically isolated work zone and bounded task.
5. Review audit receipts after each early session.
6. Promote more capability only after demonstrated safe operation.

## Explicit non-goals for initial release

- Multi-agent control.
- Automatic world reset.
- Broad third-party-mod compatibility beyond the tested set.
- Blueprint/macros that conceal primitive gameplay actions.
- Any gameplay administrative escape hatch.
