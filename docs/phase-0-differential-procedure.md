# Phase-0 manual differential procedure

## Purpose

This is the manual release gate for deciding whether the headless virtual actor
has a stable, isolated lifecycle and bounded observation model. It does not
approve gameplay writes. It records evidence for the Phase-0 Go/Pivot decision;
action-by-action equivalence is a Phase-1 promotion gate.

A failed, missing, or inconclusive probe is a blocked gate, not a pass.

## Preconditions

- Use a copied, disposable Factorio 2.x vanilla save. Never use a valued save.
- Record the Factorio version, bridge build, enabled-mod manifest, save SHA-256,
  capability-contract JSON, probe operator, and UTC start time.
- Configure the bridge actor ID before the test save is initialized. The default
  fixture actor is `alfred` and starts with the `empty` inventory policy.
- Start the dedicated headless server with no connected human client. Confirm
  the bridge reports the expected stable actor identity and no unavailable
  lifecycle state.
- Connect one real human-controlled client only for the human comparison rows.
  Keep the human identity and virtual actor identity distinct.
- Preserve the unmodified input save and use a separate copied save for each
  probe group.

## Evidence record

Create one JSON or CSV row for every attempted operation. Retain raw server and
bridge logs alongside the row data. Each row must contain:

- probe ID and actor kind (`virtual` or `human`)
- Factorio and bridge versions, enabled-mod manifest, input and output save hash
- initial and final game tick
- initial and final position
- initial and final health
- initial and final inventory, plus the item-by-item delta
- target entity identity and position when applicable
- request/action ID, result, rejection reason, and relevant observed events
- connected-player count before and after
- notes describing any deviation, timeout, manual intervention, or server error

## Probe matrix

Run each scenario first with the virtual actor, then from the same controlled
starting state with the human player. Do not adjust the virtual test conditions
to conceal a discrepancy.

| ID | Scenario | Required comparison |
|---|---|---|
| P0-01 | Zero-client status and reconnect | Actor identity, character identity, position, inventory and policy remain stable across MCP disconnect/reconnect with no connected human. |
| P0-02 | Save/reload and server restart | The same stored actor is recovered after save reload and complete server restart; no replacement entity is created. |
| P0-03 | Human isolation | With a human connected, bridge status remains bound to `alfred`; the human is not selected, moved, deleted, altered, or used as an observation origin. |
| P1-04 | Movement and collision | Phase-1 promotion gate after `walk_to` exists: compare elapsed ticks, final position and collision/reach result for an equal-distance walk into clear terrain and against an obstacle. |
| P1-05 | Mining | Phase-1 promotion gate after `mine` exists: compare target eligibility, elapsed ticks, inventory delta, target result and emitted mining events for one manually selected resource/entity. |
| P1-06 | Hand crafting | Phase-1 promotion gate after `craft` exists: compare craft eligibility, elapsed ticks, ingredient consumption, output delta and craft-related events for one unlocked recipe. |
| P1-07 | Placement | Phase-1 promotion gate after placement exists: compare reach/collision rejection and, for a valid placement, inventory delta, placed entity identity and placement events. |
| P1-08 | Inventory capacity | Phase-1 promotion gate after transfers exist: compare accepted/rejected transfer behaviour at normal capacity and at a deliberately full inventory. No items may be granted or removed outside the tested operation. |
| P1-09 | Death and respawn | Phase-1 promotion gate after death/respawn handling exists: compare health, death/corpse/inventory-loss behaviour, respawn timing and post-respawn inventory. Detect any duplication or consequence bypass. |

## Evaluation

For every row, classify the result as `equivalent`, `material deviation`, or
`inconclusive`.

A material deviation includes, at minimum:

- direct creation, mining, movement, teleportation, placement, inventory
  mutation, health restoration, or respawn behaviour that bypasses ordinary
  player mechanics;
- different reach, collision, timing, inventory, death/respawn, or relevant
  mod-event behaviour without an approved and documented explanation;
- selecting, altering, or deriving actor state from a connected human player;
- loss of stable actor identity over reconnect, save reload, or restart.

## Go/Pivot decision

Phase-0 Go is permitted when P0-01 through P0-03 have retained evidence and no
material deviation. The project remains observation-only until that decision is
recorded and reviewed. Each P1 probe must pass before promoting its corresponding
gameplay primitive.

Pivot to a permanently connected, dedicated Factorio client when any required
ordinary-player mechanic or relevant event cannot be demonstrated as equivalent
for the virtual actor. Do not weaken the definition of fair play to make a
virtual-actor implementation pass.
