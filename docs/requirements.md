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

## 3. Platform neutrality and determinism

The gameplay MCP is a general-purpose, agent-neutral game-control interface. It
provides mechanics and observations, not a strategy for completing Factorio.

- The MCP must not encode factory strategies, progression sequences, layout
  templates, task decomposition, recovery policies or decision heuristics.
- The MCP must not expose strategy-bearing macro actions such as
  `build_smelter`, `bootstrap_red_science`, `fix_factory` or
  `complete_research_chain`.
- Every supported agent must receive the same versioned capability manifest,
  schemas, validation rules, observations and action semantics.
- The MCP must not adapt tool availability, response detail, defaults or
  behaviour according to the calling model, harness, identity or prior agent
  performance.
- Convenience operations are permitted only when they are mechanically neutral,
  fully specified and do not perform hidden gameplay work, choose a strategy or
  reveal information outside the observation policy.
- Given the same Factorio build, enabled-mod manifest, save state, action
  sequence and scheduling rules, the bridge must produce equivalent observable
  results.

## 4. Character lifecycle

The MCP shall:

- Create or recover one persistent in-game character at a valid force spawn position.
- Continue operating while no Factorio client is connected.
- Preserve the same actor across MCP reconnects, server restarts, and save reloads.
- Never select or control a connected human player implicitly.
- Never delete, replace, move, or alter human characters.
- Use a documented and configurable starting-inventory policy; existing live worlds should default to an empty inventory unless the operator explicitly chooses otherwise.
- Preserve normal health, damage, armour, death, corpse, inventory-loss, and delayed-respawn consequences.

## 5. Fair-play requirements

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
- Every observation must identify whether it derives from character-local
  observation, force-charted knowledge, permitted direct interaction or
  force-level statistics.
- The MCP must not use server-global knowledge to provide a more complete or
  helpful answer than the observation policy permits.

## 6. MCP feature requirements

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

## 7. Capability contract and action lifecycle

- The MCP must publish a stable, versioned capability manifest for every
  session. It must identify the MCP schema version, fair-play policy version,
  bridge-mod build, Factorio build, enabled-mod names and versions, actor and
  force identity, enabled capability profile, observation limits, action
  scheduling rules and current game tick.
- Each mutation receipt must record the relevant schema, bridge, Factorio,
  mod-manifest and capability-profile versions.
- Mutating actions must have a unique action ID. The caller may supply an action
  ID, or the MCP must return the server-assigned ID before execution begins.
- Retrying the same action ID must be idempotent: it must return the existing
  action state or receipt and must not repeat gameplay work.
- The initial one-actor release must serialize mutating actions in a documented
  order. An action’s scheduling point must be defined in game ticks.
- Actions must expose one of the states `accepted`, `running`, `succeeded`,
  `rejected`, `cancelled` or `failed`.
- Cancellation and `stop` semantics must be documented, including the effect on
  partially completed ordinary gameplay actions.
- Rejection and failure responses must use stable structured codes in addition
  to human-readable detail.
- Action state and receipt history must be queryable by action ID.

## 8. Prohibited gameplay capabilities

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

## 9. Safety and auditability

- Every mutating action shall produce an audit record containing actor ID, action ID, arguments, start/end tick, start/end position, health, inventory delta, affected entity IDs, result, fair-play policy version and the version identifiers required by section 7.
- Audit records should make alteration or truncation apparent.
- Credentials and unrelated player communications must never enter audit output.
- Risky development and integration tests must use disposable save copies.
- Live deployment requires a save backup and documented rollback procedure.

## 10. Verification requirements

Candidate solutions must demonstrate:

- A stable, reviewable MCP capability manifest containing no prohibited gameplay tools.
- Tests that prove the published capability contract is identical for every
  supported agent identity and that retrying a mutation with the same action ID
  is idempotent.
- Evidence that prohibited administrative shortcuts are absent from gameplay paths.
- Item-conservation tests for every transfer and construction path.
- Headless tests with zero connected clients.
- Save/restart and actor-persistence tests.
- Tests proving connected humans remain untouched and cannot become the MCP actor.
- Differential tests comparing movement, mining, crafting, placement, reach, collision, inventory, and event behaviour with a real human-controlled character.
- Death and respawn tests that detect item duplication or consequence bypass.
- An unattended end-to-end test that builds a factory and measures at least 12 iron gears during each of two consecutive 60-game-second windows.

## 11. Equivalence requirement

A candidate solution is acceptable only if testing demonstrates sufficiently normal player mechanics. This includes gameplay results and relevant events observed by the declared, enabled mod set.

The mechanism used to provide the character is not prescribed. If material gameplay or mod-event equivalence cannot be demonstrated, the solution does not meet these requirements. The definition of fair play must not be weakened to accommodate a particular implementation. Compatibility with any third-party mod requires that mod and version to be named in the enabled-mod manifest and covered by differential tests; the initial release makes no universal third-party-mod compatibility claim.

## 12. Initial release boundary

The first release targets:

- Factorio 2.x vanilla mechanics.
- One persistent MCP character.
- One existing multiplayer force and world.
- Safe coexistence with human players.
- A capability coverage matrix for the ordinary interactions required on the
  vanilla path to launching a rocket.

Multi-agent operation, automatic world resets, and broad third-party mod compatibility are deferred.
