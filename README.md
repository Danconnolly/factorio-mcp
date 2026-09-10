# Factorio Agent Bridge

Factorio Agent Bridge is a Factorio 2.x mod and Model Context Protocol (MCP)
project for an AI harness to inspect and operate one dedicated, persistent
in-game character without granting it administrative shortcuts.

The project is designed for safe coexistence with human players in a persistent
world. Fair play is an acceptance criterion, not a marketing adjective: the
actor must demonstrate normal movement, mining, crafting, building, inventory,
research, death and respawn consequences, or the implementation does not qualify.

## Status

Phase 0 complete: read-only foundation and feasibility gate. The repository
contains the requirements, open-source assessment, implementation plan, tested
Factorio 2.1 bridge shell, versioned capability contract, bounded read-only MCP
service and tamper-evident audit sink. Automated disposable-fixture gates and
the manual real-client human-isolation check passed; see
[Phase-0 completion record](docs/phase-0-completion.md).

Phase 1 / walk-stop complete: the bounded `walk_to` and `stop` slice passed its
automated disposable fixture and manual real-client movement/collision
differential gate. See the [Phase-1 walk/stop completion
record](docs/phase-1-walk-stop-completion.md). Only this primitive has been
promoted; no gameplay controller should be attached to a valued save at this
stage.

Phase 1 / mine complete: the tick-driven, inventory-conserving `mine` action
passed its copied-save zero-client fixture and final manual real-player
differential, including the Factorio 2.x quality-aware inventory path. See the
[Phase-1 mine completion record](docs/phase-1-mine-completion.md). Only
walk/stop and mine are promoted; no gameplay controller should be attached to a
valued save at this stage.

Phase 1 / craft feasibility gate: the disposable zero-client probe established
that the virtual headless actor has no native hand-crafting queue. `craft` is
therefore fail-closed and not promoted; the fair-play route for it is a
permanently connected dedicated Factorio client. See the [Phase-1 craft
feasibility record](docs/phase-1-craft-feasibility.md).

That finding invalidates the original all-headless route to a complete
controller. The refreshed open-source assessment identifies FactoMCP as a
possible *fork base* for a strict, connected-player bridge, but no existing
project is safe to adopt unchanged; see the [connected-player
re-evaluation](docs/open-source-re-evaluation.md).

## Design constraints

- One configured, persistent MCP actor; it must never implicitly select or alter
  a human player.
- Connected-player operation: the MCP actor is a separately configured,
  permanently connected dedicated Factorio client. It must survive MCP
  reconnects, game restarts and save reloads without selecting a human player.
- No gameplay teleportation, raw Lua/RCON execution, item grants, instant
  mining/crafting/pickup, free placement, research manipulation, reset, speed
  control or unbounded map reveal.
- Game-time actions must respect reach, collision, inventory capacity, item
  conservation and normal timing.
- Every mutation produces a structured, tamper-evident audit receipt.
- The MCP is agent-neutral: it exposes fair mechanics and observations, not a
  preferred factory strategy, layout, progression path or agent-specific tool
  privilege.
- Sessions publish a versioned capability contract so the same action sequence
  can be interpreted and replayed against a defined Factorio/mod/tool profile.
- Development and integration testing use disposable saves. Live deployment
  requires a backup and a tested rollback procedure.

## Documentation

- [Requirements](docs/requirements.md) — product requirements and executable
  acceptance criteria.
- [Open-source scan](docs/open-source-scan.md) — assessment of existing
  Factorio/MCP projects against the requirements.
- [Connected-player re-evaluation](docs/open-source-re-evaluation.md) —
  post-craft-gate candidate ranking and strict-fork acceptance plan.
- [Implementation plan](docs/implementation-plan.md) — phased delivery plan,
  including the initial feasibility gate and release verification suite.
- [Phase-0 differential procedure](docs/phase-0-differential-procedure.md) —
  mandatory manual evidence and Go/Pivot decision procedure.
- [Phase-0 completion record](docs/phase-0-completion.md) — retained summary of
  the completed read-only feasibility gate and Phase-1 promotion boundary.
- [Phase-1 walk/stop completion record](docs/phase-1-walk-stop-completion.md)
  — retained, sanitized evidence for the P1-04 movement promotion gate.
- [Phase-1 mine completion record](docs/phase-1-mine-completion.md)
  — retained, sanitized evidence for the mining promotion gate.
- [Phase-1 craft feasibility record](docs/phase-1-craft-feasibility.md)
  — zero-client gate result and required connected-client pivot.
- [Optional Kubernetes fixture](docs/kubernetes-fixture.md) — opt-in,
  default-scheduled disposable-server provider for the ignored integration
  checks; it is not required for unit tests.

## Implementation direction

The target implementation is a strict Factorio bridge mod plus a typed MCP
service. The bridge mod owns actor lifecycle and tick-driven action state; the
MCP service validates typed requests and does not expose arbitrary RCON or Lua.

The first milestone is a read-only, headless actor lifecycle and differential
test harness. A virtual character may proceed to gameplay tools only after tests
show it can satisfy the documented mechanics. If the required equivalence cannot
be demonstrated, the project will pivot to a dedicated connected Factorio client
rather than weakening the fair-play definition.

This repository is the controller layer. A separate harness repository will own
agent adapters, scenarios, benchmarks, budgets, scorecards and the eventual
vanilla rocket-launch comparison runs.

## Developer prerequisites

- Rust 1.98.1 (pinned in `rust-toolchain.toml`) with Cargo, rustfmt and Clippy.
- A Factorio 2.x headless-server distribution before running integration tests.
  Factorio provides a free Linux headless server; the fixture must use a
  disposable copy of a save.
- An RCON endpoint and password only when later bridge integration is enabled;
  keep them outside the repository, for example in a local `.env` file.

The host-side controller is Rust. The small runtime bridge is Lua because that
is Factorio's mod-control interface. The future Rust RCON adapter will call a
closed set of named bridge functions; it will not provide a generic Lua or RCON
console.

Current host checks:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

These checks never require a Factorio server, Kubernetes access, an RCON
credential, or a save file. The lifecycle and observation tests are explicit
`#[ignore]` integration gates; run them only through an operator-provided,
disposable fixture as documented in
[Optional Kubernetes fixture](docs/kubernetes-fixture.md).

The presence of this skeleton is not evidence of virtual-character equivalence
or of fair play. Those claims remain gated on the documented Phase-0 lifecycle,
observation and manual real-player differential tests. The latter are a release
gate, not part of CI or automated deployment.

## License

[MIT](LICENSE), © 2026 Daniel Connolly.
