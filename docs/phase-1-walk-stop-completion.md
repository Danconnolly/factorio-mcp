# Phase-1 walk/stop completion record

## Decision

**P1-04 passed.** The strict `walk_to` and `stop` vertical slice is promoted.
The next gameplay primitive is `mine`, subject to its own conservation and
real-player differential gate.

## Evidence retained

- Automated zero-client, disposable Kubernetes fixture:
  `test-reports/factorio-fixture-cade0ffd/report.json`
  - Factorio `2.1.17`, bridge/policy `0.2.0`, actor `alfred` (unit `7`).
  - A clear walk completed as `TARGET_REACHED` from tick `2` to `21`.
  - A longer walk was cancelled as `STOP_REQUESTED`, and the serialized stop
    completed as `STOPPED`.
  - Input save SHA-256:
    `a72cd14b7ef272c330cf4a5d60dac2b89e85f9f8ba5347d1223d9a8cdbb01ceb`.
- Manual real-client differential:
  `test-reports/phase-one-manual-client-differential.md`
  - Factorio `2.1.17` build `87315`; bridge `0.2.0`; disposable copied save.
  - Connected human `otaci` observed `alfred` perform a clear-terrain walk,
    halt after `stop`, and fail as `BLOCKED` at three legitimately built wooden
    chests.
  - The human then walked into the centre chest and observed the same collision
    boundary.

The raw reports remain ignored local evidence because they include disposable
fixture metadata and may include sensitive operational output. This document is
the retained, sanitized promotion record.

## Gate result

The movement implementation demonstrated:

- tick-driven target completion;
- durable `TARGET_REACHED`, `STOP_REQUESTED`, and `STOPPED` receipts;
- serialized cancellation that clears movement input;
- collision against ordinary player-built entities; and
- no human-player selection or privileged movement path during the probe.

These results meet the P1-04 comparison requirements in
[`phase-0-differential-procedure.md`](phase-0-differential-procedure.md).
They establish only the walk/stop primitive; they do not imply equivalence for
mining, crafting, placement, transfer, death, or mod-event behaviour.

## Observation transport resolution

The manual-session raw report records an earlier `scan_local` RCON
`error: invalid data` at radii `4`, `8`, and `32`. It is historical evidence and
is deliberately preserved unchanged. The issue was subsequently fixed by
bridge-side deterministic response bounding: the result is truncated before its
encoded JSON exceeds the published 3072-byte payload limit, leaving envelope
headroom for Factorio's RCON console framing.

The disposable observation fixture retained at
`test-reports/factorio-fixture-0de69809/report.json` proves the fix against a
maximum-radius (`32`) local scan: it completed successfully with `partial` and
`truncated` set, `result_count` `28`, and `payload_bytes` `3039`, below the
3072-byte bridge limit. The same fixture rejected radius `33` as
`OUT_OF_POLICY` and an uncharted request as `UNCHARTED`.

The bridge's current observation contract and deterministic truncation rules
are documented in
[`crates/factorio-agent-contract/docs/protocol.md`](../crates/factorio-agent-contract/docs/protocol.md).
