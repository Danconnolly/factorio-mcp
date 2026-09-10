# Phase-1 craft feasibility record

## Decision

**Do not promote virtual-headless `craft`.** The Phase-1 zero-client feasibility
probe found that the bridge-owned virtual character has no native Factorio
hand-crafting queue. The project must not simulate crafting by removing inputs,
waiting, and inserting products.

The implementation plan's pivot condition applies: any future fair-play
hand-crafting primitive requires a permanently connected dedicated Factorio
client represented by a real `LuaPlayer`. Until that actor model exists and
passes its own disposable and real-player differential gates, the headless
virtual actor must reject `craft` as `OUT_OF_POLICY`.

## Probe evidence

- Disposable copied save:
  `test-reports/fixture-seed.zip`
- Input SHA-256:
  `83cb67e5b1a34d131a6ced4ba52ca27f660a8259ce663cde4280a6fbe2ecf259`
- Automated evidence bundle:
  `test-reports/factorio-fixture-bfb72685/report.json`
- Factorio `2.1.17`; bridge/policy `0.4.0`; actor `alfred` (unit `7`).
- The server reached `InGame` with zero connected humans. The actor had an empty
  inventory and a stable normal lifecycle.
- The fixed `craft` request for one `iron-gear-wheel` was rejected before any
  inventory or world mutation:

  ```text
  OUT_OF_POLICY: the configured virtual actor has no native hand-crafting queue;
  use a connected player actor
  ```

- The fixture passed, and its `finally` cleanup removed the disposable namespace
  and Factorio Pod. The raw server log captured the original nil-queue exception
  during discovery; the bridge now detects the missing native queue and fails
  closed instead.

## Why no virtual crafting approximation

A bridge-side approximation that validates ingredients, removes them, waits, and
inserts products would not be ordinary Factorio crafting. It could diverge from
native queue ordering, prerequisite/intermediate handling, partial completion,
cancellation/refunds, inventory-full behaviour, modifiers, and player/mod event
semantics. It would violate the requirements' prohibition on instant or
synthetic crafting and would turn a failed equivalence gate into an untested
claim.

If an inventory-conserving approximation is ever wanted, it must be a separately
named product mode with its own policy version and differential evidence. It is
not a substitute for the fair-play gameplay MCP.

## Scope after the gate

- Promoted virtual-headless primitives remain `walk_to`, `stop`, and `mine`.
- `craft` is retained only as an explicit, fail-closed capability while the
  client-backed actor model is designed; it is not a promoted gameplay action.
- Placement, transfers, research, and all later Phase-1 primitives remain
  blocked on their individual feasibility gates.
