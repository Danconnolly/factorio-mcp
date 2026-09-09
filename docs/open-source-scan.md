# Open-source scan: Factorio Fair-Play MCP

Scope: compared publicly available Factorio/MCP repositories and mods against [`requirements.md`](requirements.md), with particular weight on: a persistent distinct actor, zero-human-client operation, normal movement/mining/crafting/building, no gameplay admin escape hatch, bounded vision, auditability, and demonstrated tests.

## Verdict

No located project currently meets the requirements in full or presents the required verification evidence.

The recurring technical fault is that an unattached `character` entity is easy to create and manipulate from Lua/RCON, but it is not an ordinary connected `LuaPlayer`; projects therefore use direct entity creation/mining or teleportation instead of equivalent player actions.

## Closest projects, but not acceptable as-is

| Project | What exists | Why it fails the fair-play requirements |
|---|---|---|
| [thedemon117/ai-player-v3](https://github.com/thedemon117/ai-player-v3) | Active Factorio 2.0 mod plus optional MCP bridge. It creates a dedicated, persistent character entity, has read queries and primitive/skill actions, and is explicitly intended for solo/co-op AI play. | The source uses `character.teleport`, direct `character.mine_entity(..., true)`, and `surface.create_entity` for action paths; its MCP also exposes raw `run_lua`. Its own mod-page description acknowledges RCON cheating/illegal placement. It is the strongest conceptual starting point, but requires replacement of the core action implementation and a restricted gameplay capability manifest. |
| [phenderson0/factorio_mods](https://github.com/phenderson0/factorio_mods) | A custom mod creates a separate AI character and an MCP server exposes movement, surroundings, mining, crafting, building, inventory transfers, and research. | Its control code explicitly states mining is instant and implements `mine_entity`; the observable code creates an actor entity directly. It has no published evidence for reach/timing, normal crafting/building event equivalence, persistence/restart, human isolation, audit receipts, or the requested end-to-end production test. |
| [MarkMcCaskey/factorioctl](https://github.com/MarkMcCaskey/factorioctl) | Broad and useful Rust MCP/CLI surface, diagnostics and planning helpers, plus a fallback virtual character for some headless paths. | Its public MCP includes raw `execute_lua`, teleport and admin-style helpers; source inspection shows direct `mine_entity` and `surface.create_entity` paths. It also hard-codes player-index access in player-centric paths, so it cannot demonstrate safe, stable selection of a dedicated actor in a shared world. |

## Realistic-player MCPs that need a connected human

- [sbarisic/FactorioMCP](https://github.com/sbarisic/FactorioMCP) is the most serious direct claim of “walking, crafting, building—without cheating”; it uses movement state and normal crafting timing. However, its action code resolves `game.connected_players[1]`, and its published tool surface contains unrestricted `ExecuteLua` plus ghost placement that does not require inventory. It therefore fails the persistent headless actor requirement and the prohibited-capability rule.
- [WidAmi/FactoMCP](https://github.com/WidAmi/FactoMCP) similarly uses `game.players[1]`, offers normal-looking walking/building APIs, and candidly says fairness is not guaranteed. It exposes unrestricted `run_lua`, so it cannot be a fair-play gameplay endpoint.
- [bbeaudreault/factorio-mcp](https://github.com/bbeaudreault/factorio-mcp) targets an LLM playing alongside humans but deliberately includes player teleportation and building on behalf of a named player. It is an RCON controller, not a dedicated normal-player implementation.

## Related but not a candidate

- [Factorio Learning Environment (FLE)](https://github.com/JackHopkins/factorio-learning-environment) is the largest and most actively maintained Factorio agent framework, including MCP support and Factorio 2.x work. Its standard model uses virtual characters and supplies benchmark/admin capabilities such as reset, inventory setup, speed control and direct mining; it is an evaluation environment, not a safe adapter for a persistent human world.
- [PlasmaChroma/FactorioMCP](https://github.com/PlasmaChroma/FactorioMCP) is unusually disciplined on bounded observation, operator policy, durable sessions and audited receipts. Its current milestone keeps gameplay writes unavailable, and its own specification makes legitimate survival play a later, separate capability. This is good architectural material for observation/audit, but it is not a working playable character.
- [lveillard/factorio-ai-companion](https://github.com/lveillard/factorio-ai-companion) has an AI-companion mod and MCP framing, but the mod portal describes it as beta, says most actions do not yet work properly, and classifies it under cheats. The implementation uses created character entities and direct placement.
- [Factorio Live Mod Agent](https://mods.factorio.com/mod/flma) is a sensible complement, not a controller: it exports read-only live state to files and deliberately has no game-side network/control path.

## Recommendation

Do not adopt any project directly into a valued multiplayer save under the fair-play name.

The productive route is a new strict bridge. Reuse or take inspiration from existing projects only where their implementations meet the required boundary: actor-lifecycle ideas from `ai-player-v3`, and bounded observation/audit architecture from `PlasmaChroma/FactorioMCP`. Discard their teleport, direct-mine, direct-create, raw-Lua and administrative gameplay paths.

Before declaring completion, a compliant implementation needs: a configured stable actor identity; headless restart/save persistence; game-time queued actions with normal reach/collision/inventory consequences; a gameplay MCP with no raw Lua/admin tools; chart-bounded observations; tamper-evident mutation receipts; and the differential, human-isolation, death/respawn, and two-window gear-production tests in the requirements.

## Sources

1. https://github.com/sbarisic/FactorioMCP
2. https://github.com/MarkMcCaskey/factorioctl
3. https://github.com/WidAmi/FactoMCP
4. https://github.com/JackHopkins/factorio-learning-environment
5. https://github.com/dested/factorio-mcp-server
6. https://github.com/thedemon117/ai-player-v3
7. https://github.com/lveillard/factorio-ai-companion
8. https://mods.factorio.com/mod/ai-player-v3
9. https://mods.factorio.com/mod/ai-companion
10. https://mods.factorio.com/mod/flma
11. https://github.com/phenderson0/factorio_mods
12. https://github.com/jerome3o/factorio-mcp
13. https://github.com/Bodewes/FactorioMcp
14. https://github.com/bbeaudreault/factorio-mcp
15. https://github.com/PlasmaChroma/FactorioMCP
