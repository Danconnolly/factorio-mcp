# Phase-1 mine completion record

## Decision

**Phase-1 `mine` passed and is promoted.** The next gameplay primitive is
`craft`, which requires its own receipt, copied-save headless fixture, and
manual real-player differential gate.

## Evidence retained

- Automated zero-client disposable Kubernetes fixture:
  `test-reports/factorio-fixture-b6865e65/report.json`
  - Factorio `2.1.17`, bridge/policy `0.3.0`, actor `alfred` (unit `7`).
  - Input copied-save SHA-256:
    `83cb67e5b1a34d131a6ced4ba52ca27f660a8259ce663cde4280a6fbe2ecf259`.
  - An out-of-reach target was rejected as `OUT_OF_POLICY` before action
    creation.
  - Ordinary character mining at `(-45.5, -45.5)` ran from tick `664` to
    `785`, produced exactly one normal iron ore, and reduced the exact target
    from `490` to `489`.
  - An identical action-ID retry returned the original receipt; conflicting
    reuse was rejected as `ACTION_ID_CONFLICT`; a distinct mining action was
    cancelled by `stop` with `STOP_REQUESTED` and no inventory delta.
- Final manual real-client differential, revalidated after the Factorio 2.x
  quality-aware inventory fix:
  `test-reports/phase-one-mine-manual-run.md`
  - Factorio `2.1.17`, bridge/policy `0.3.0`, disposable copied save.
  - Connected human `otaci` (unit `11`) normally mined one separate iron tile
    and confirmed one iron ore entered the human inventory.
  - `alfred` (unit `7`) mined the reserved tile at `(-45.5, -45.5)` from tick
    `11266` to `11387`, with receipt inventory delta `+1 normal iron-ore` and
    target depletion `490 -> 489`.
  - The manual probe also proved reach rejection, identical-retry safety,
    conflicting-ID rejection, cancellation with no inventory delta, and Daniel
    directly witnessed Alfred mine.

The raw reports remain ignored local evidence because they contain disposable
fixture metadata and operational logs. This document is the retained sanitized
promotion record.

## Gate result

The mine implementation demonstrated:

- native tick-driven `character.mining_state` rather than direct mining;
- ordinary reach and selected-target validation before acknowledgement;
- elapsed game ticks, stable actor position and stable health;
- exact quality-aware inventory conservation and resource depletion;
- durable action IDs, idempotent retry, and conflict rejection;
- serialized stop cancellation; and
- comparable connected-player evidence using a separate resource tile.

The final differential specifically exercised the corrected Factorio 2.x
`LuaInventory.get_contents()` quality-record path with a non-empty actor
inventory. Both required promotion gates now validate the same bridge build.

These results promote only `mine`. They do not establish equivalence for
crafting, placement, transfers, inventory-capacity interaction, research,
death/respawn, or relevant mod events.
