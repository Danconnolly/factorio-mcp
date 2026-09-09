# Factorio Fair-Play MCP Requirements

## Status

This document defines the acceptance requirements for Factorio MCP. A solution is not fair-play merely because it resembles normal play or omits an obvious cheat command: it must satisfy the requirements and demonstrate the required verification evidence.

## 1. Purpose

Provide an MCP server through which an AI agent can inspect and play a persistent Factorio 2.x world without requiring a human client to remain connected.

The controlled character must behave like a normal player. These requirements describe required observable behaviour and capabilities without prescribing an implementation.

## 2. Operating model

- The MCP controls one dedicated, persistent in-game character with a stable identity.
- The character remains available while no human player is connected.
- The character and its state survive MCP reconnects, game-server restarts, and save reloads.
- Human and MCP-controlled characters remain distinct at all times.
- Gameplay capabilities and operator-only administrative capabilities are strictly separated.

## 3. Character lifecycle

The MCP shall:

- Create or recover one persistent in-game character at a valid force spawn position.
- Continue operating while no Factorio client is connected.
- Preserve the same actor across MCP reconnects, server restarts, and save reloads.
- Never select or control a connected human player implicitly.
- Never delete, replace, move, or alter human characters.
- Use a documented and configurable starting-inventory policy; existing live worlds should default to an empty inventory unless the operator explicitly chooses otherwise.
- Preserve normal health, damage, armour, death, corpse, inventory-loss, and delayed-respawn consequences.

## 4. Fair-play requirements

### Movement

- Movement must unfold through ordinary in-game character movement and consume normal game time.
- Respect collision, terrain, movement speed, equipment modifiers, obstruction, and hazards.
- No teleportation during gameplay.

### Mining and collection

- Mining must unfold as ordinary character mining and consume normal game time.
- Respect reach, mining time, mining-speed modifiers, mineability, target identity, and inventory capacity.
- Do not directly decrement resources, mine entities instantly, or synthesize mining output.

### Crafting

- Use Factorio’s ordinary hand-crafting queue and completion timing.
- Respect recipe availability, researched technologies, ingredients, intermediate products, crafting categories, speed, and inventory capacity.
- No instant or recursive synthetic crafting.

### Building and interaction

- Require the relevant item in character inventory before placement.
- Enforce normal build distance, collision, direction, surface, force, and placement validity.
- Consume exactly one item only after successful placement.
- Enforce interaction reach for insertion, extraction, rotation, recipe selection, loading, unloading, and pickup.
- Preserve item conservation across the actor, ground, and machine inventories.
- Support specialised valid-placement logic where necessary, including offshore pumps.

### Research and time

- Use the actor’s ordinary force recipes, technologies, laboratories, and science packs.
- Do not grant, reset, or bypass research.
- Measure action duration using the game’s actual simulation time.
- Do not pause time, advance time synthetically, or change game speed.

### Observation

- Allow local inspection around the character and wider inspection only in chunks already charted by the actor’s force.
- Do not reveal resources or enemies in uncharted terrain.
- Do not generate chunks merely to answer a query.
- Force-shared map knowledge legitimately charted by allied human players remains available.

## 5. MCP feature requirements

### Character and world state

- Character status, position, health, inventory, equipment, and crafting queue.
- Local area scan and detailed tile/entity view.
- Entity status by position or stable entity identifier.
- Recipe, prototype, technology, and research status queries.
- Resource-patch discovery within permitted observation bounds.
- Factory diagnostics, alerts, belt contents, fluid connections, and production statistics.
- Screenshots or rendered local views where supported.

### Gameplay actions

- Walk to a position and stop current action.
- Mine resources, trees, rocks, and recoverable entities.
- Hand-craft unlocked recipes.
- Place one entity or a validated line of entities.
- Rotate entities and set machine recipes.
- Insert items into and extract items from entity inventories.
- Inspect and operate belts, inserters, assemblers, furnaces, boilers, generators, laboratories, pipes, pumps, chests, and related vanilla entities.
- Select ordinary research targets.
- Wait for and measure real production.

All actions shall return structured results containing success or rejection reason, actor identity, positions, elapsed ticks, inventory changes, and affected entities.

## 6. Prohibited gameplay capabilities

The gameplay MCP must not expose:

- Arbitrary scripting or unrestricted game-administration execution.
- Teleportation.
- Direct item grants or inventory replacement.
- Instant mining, crafting, or entity pickup.
- Free entity creation or placement without item consumption.
- Technology grants or resets.
- World/entity clearing, save resets, or resource regeneration.
- Game-speed, tick, or pause manipulation.
- Unbounded map inspection or forced chunk generation.

Any operator maintenance capability must be separate, disabled by default, and inaccessible through the gameplay MCP.

## 7. Safety and auditability

- Every mutating action shall produce an audit record containing actor ID, action ID, arguments, start/end tick, start/end position, health, inventory delta, affected entity IDs, result, and fair-play policy version.
- Audit records should make alteration or truncation apparent.
- Credentials and unrelated player communications must never enter audit output.
- Risky development and integration tests must use disposable save copies.
- Live deployment requires a save backup and documented rollback procedure.

## 8. Verification requirements

Candidate solutions must demonstrate:

- A stable, reviewable MCP capability manifest containing no prohibited gameplay tools.
- Evidence that prohibited administrative shortcuts are absent from gameplay paths.
- Item-conservation tests for every transfer and construction path.
- Headless tests with zero connected clients.
- Save/restart and actor-persistence tests.
- Tests proving connected humans remain untouched and cannot become the MCP actor.
- Differential tests comparing movement, mining, crafting, placement, reach, collision, inventory, and event behaviour with a real human-controlled character.
- Death and respawn tests that detect item duplication or consequence bypass.
- An unattended end-to-end test that builds a factory and measures at least 12 iron gears during each of two consecutive 60-game-second windows.

## 9. Equivalence requirement

A candidate solution is acceptable only if testing demonstrates sufficiently normal player mechanics. This includes gameplay results and relevant events observed by other installed mods.

The mechanism used to provide the character is not prescribed. If material gameplay or mod-event equivalence cannot be demonstrated, the solution does not meet these requirements. The definition of fair play must not be weakened to accommodate a particular implementation.

## 10. Initial release boundary

The first release targets:

- Factorio 2.x vanilla mechanics.
- One persistent MCP character.
- One existing multiplayer force and world.
- Safe coexistence with human players.

Multi-agent operation, automatic world resets, and broad third-party mod compatibility are deferred.
