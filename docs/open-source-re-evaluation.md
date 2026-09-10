# Open-source re-evaluation: connected-player pivot

## Trigger and decision boundary

The Phase-1 craft feasibility probe established that the bridge-owned,
zero-client virtual `character` does not expose Factorio's native hand-crafting
queue. The project must not emulate that queue with bridge-side inventory
mutation or timing. Consequently, a complete fair-play controller now requires
a **permanently connected dedicated Factorio client** represented by a real
`LuaPlayer`.

This is a re-evaluation of the candidates in
[open-source-scan.md](open-source-scan.md), not a relaxation of the fair-play
requirements. A candidate remains unacceptable as a gameplay MCP if it exposes
arbitrary Lua/RCON or gameplay administration, teleports, directly mines or
creates entities, permits switching to a human player, or lacks evidence for
its claimed mechanics.

## Outcome

No inspected project is safe to adopt unchanged for a persistent human world.
However, the connected-client pivot changes the ranking materially:
**WidAmi/FactoMCP is now the best narrow fork base**. Its normal action paths
are built around a real `LuaPlayer` and native Factorio operations, whereas the
former virtual-actor candidates remain fundamentally mismatched.

The recommended next decision is therefore not to continue extending this
repository's virtual-headless actor. Choose between:

1. fork FactoMCP into a strict, single-actor gameplay bridge; or
2. continue the existing bridge only after replacing its actor model with the
   same permanently connected dedicated-player architecture.

Neither route may keep a public raw-Lua recovery tool or simulate crafting.

## Candidate assessment

| Rank | Project | What now fits | Blocking defects | Recommendation |
|---|---|---|---|---|
| 1 | [WidAmi/FactoMCP][1] | Real `LuaPlayer`; native `walking_state`, `mining_state`, `begin_crafting`, crafting queue, and `build_from_cursor`. | Every action resolves `game.players[1]`; public `run_lua` submits arbitrary `/silent-command` Lua; direct inventory-transfer paths remain; no test suite or lifecycle/isolation evidence was found. | **Fork candidate.** Replace the actor resolver with a configured immutable dedicated player, delete `run_lua`, close dispatch in a mod, and add the missing evidence gates. |
| 2 | [sbarisic/FactorioMCP][2] [3] | Native `LuaPlayer.begin_crafting` and native queue observation; resource mining uses player mining state; larger conventional host test suite than the other candidates. | Selects `game.connected_players[1]`; public `ExecuteLua` explicitly has full unsandboxed Lua access; direct `surface.create_entity`, direct building mining, ghost operations, and unbounded caller-directed vision remain. | **Reference / deep-fork only.** Retain native craft/mining concepts and test structure, but removing unsafe surface area is a substantial rewrite. |
| 3 | [bbeaudreault/factorio-mcp][4] [8] | Caller-supplied player name could be evolved into a fixed dedicated identity; its mod separates named query/action routes. | Caller can name any player; gameplay actions are teleport and direct `surface.create_entity` plus item removal; no craft/mining route; host tests do not prove in-game mechanics. | **Reuse architectural idea only.** The closed mod-dispatch split is useful; its gameplay implementation is not. |
| 4 | [MarkMcCaskey/factorioctl][5] | Native character crafting and a broad Rust MCP/CLI implementation. | Chooses the first valid connected player, with a virtual-character fallback; exposes generic Lua, teleportation, immediate `mine_entity`, area clearing, and direct virtual-actor creation. | **Reference only.** It is not safely hardened by configuration. |
| 5 | [thedemon117/ai-player-v3][6] | Persistent virtual-actor lifecycle and some bounded-query ideas. | The actor is an unattached entity; its core movement is teleportation, mining is direct, placement uses `surface.create_entity`, and raw Lua is public. | **No controller reuse.** The actor model and action core conflict with the pivot. |
| 6 | [phenderson0/factorio_mods][7] | Stored virtual-actor reference and fixed remote-interface names. | Virtual actor; instant `mine_entity`; direct entity creation and item removal; no connected-player equivalence evidence. | **Reference only.** Do not adopt the actor or gameplay paths. |

Other previously reviewed projects remain non-candidates: FLE is deliberately an
evaluation environment with administrative/reset capabilities; generic
RCON/Lua wrappers such as Jerome's and Bodewes' projects are administration
surfaces, not embodied gameplay bridges; and the remaining virtual-character
projects have the same failed craft premise.

## Why FactoMCP is the leading fork base

FactoMCP is the only inspected candidate whose ordinary action implementations
already point in the desired direction: `game.players[1]` drives movement and
mining state, hand crafting calls `begin_crafting`, and placement uses
`build_from_cursor` rather than spawning the final entity directly.[1]

That is not a deployment endorsement. `game.players[1]` is unsafe in a shared
world because connection/index order can select a human. Its unrestricted
`run_lua` makes every alleged gameplay restriction bypassable. These are
architectural defects, but they are concentrated enough that a deliberately
narrow fork is likely less work and less risk than retrofitting the present
headless virtual-actor controller into a connected-client system.

## Minimum fork acceptance plan

A chosen fork must begin with a deletion-and-isolation milestone, before adding
or retaining any later gameplay capabilities:

1. Remove arbitrary Lua, generic RCON command execution, teleport, grants,
   direct mine/create/destroy operations, ghost/blueprint administrative tools,
   unbounded scans, and any caller-selectable player parameter from the public
   gameplay MCP.
2. Introduce a configured dedicated player name and validate on every bridge
   call that it resolves to one connected `LuaPlayer` with a character. Reject
   absence, ambiguity, or human identity mismatch; never fall back to an index
   or virtual entity.
3. Put game access behind a small Factorio mod with fixed query and command
   methods. The host service may transmit typed data to named methods, but may
   not forward caller-provided Lua or `remote.call` targets.
4. Keep only native action primitives: tick-driven walking/mining state,
   `LuaPlayer.begin_crafting` and native queue observation/cancellation,
   `build_from_cursor`, and ordinary inventory/entity interaction paths proven
   against a human control.
5. Add the existing project requirements for capability manifest, action IDs,
   serialized lifecycle, durable tamper-evident receipts, chart-bounded
   observations, item conservation, save/restart behavior, and explicit
   cancellation semantics.
6. Run disposable connected-client and real-player differential gates for each
   primitive, including named-actor isolation, crafting queue timing and
   cancellation, placement/inventory conservation, death/respawn, and the
   two-window gear-production test.

## Revised conclusion

The original conclusion—no headless virtual actor can yet satisfy the full
fair-play contract—stands. The new conclusion is narrower and actionable:
there is no adoption-ready open-source solution, but a dedicated connected
player makes FactoMCP's native-player primitive layer a plausible fork base.

The project should make an explicit **fork-versus-rebuild** decision before
further Phase-1 work. Continuing virtual-headless primitives beyond the craft
gate cannot reach the stated vanilla rocket-path goal.

## Sources

[1] https://github.com/WidAmi/FactoMCP/blob/f223f5482801638fe0f930ab84a073b6e5ac8ca1/server.py — FactoMCP server.py at f223f54
[2] https://github.com/sbarisic/FactorioMCP/blob/f2fca61707efe3107e7a3738bb83d1505b0126f7/FactorioMCP/Services/FactorioService.Inventory.cs — FactorioMCP native crafting implementation
[3] https://github.com/sbarisic/FactorioMCP/blob/f2fca61707efe3107e7a3738bb83d1505b0126f7/FactorioMCP/Tools/LuaTools.cs — FactorioMCP arbitrary Lua MCP tool
[4] https://github.com/bbeaudreault/factorio-mcp/blob/e607490f6c9f88457fb77831dea0914b556154b7/src/factorio_mcp/server.py — bbeaudreault Factorio MCP server
[5] https://github.com/MarkMcCaskey/factorioctl/blob/c3abe9ea099b549e18b149d0ea483de21a73bdbe/src/client/lua.rs — factorioctl Lua command implementations
[6] https://github.com/thedemon117/ai-player-v3/blob/2806aed808a1966db21f4edebc8060dd30cc332b/mod/scripts/primitives.lua — ai-player-v3 gameplay primitives
[7] https://github.com/phenderson0/factorio_mods/blob/c7611122c368a3947af8d203d5a42843dcab423f/ai_character_0.1.0/control.lua — phenderson Factorio actor mod
[8] https://github.com/bbeaudreault/factorio-mcp/blob/e607490f6c9f88457fb77831dea0914b556154b7/factorio_mod/mcp-controller/control.lua — bbeaudreault closed Factorio mod dispatch
