# Phase-0 completion record

## Status

Phase 0, the read-only feasibility and foundation phase, is complete. The
project is approved to begin Phase 1 strict primitive gameplay actions; it is
not approved for gameplay use on a valued save.

## Retained evidence

- The normal workspace suite passes without a Factorio server, Kubernetes,
  RCON credentials or a disposable save.
- The opt-in lifecycle and observation integration gates passed against the
  default-scheduled, disposable Kubernetes fixture.
- A disposable Factorio 2.1.17 server loaded bridge build `0.1.0`, reached
  `InGame`, and returned the expected named bridge status for actor `alfred`.
- The manual real-client human-isolation check passed: a normally connected
  player received a distinct character entity while the stored `alfred` actor
  retained its recorded unit identity and position. No human console, RCON,
  administrative or gameplay action was used for that check.
- The disposable manual server and temporary mod-download service were deleted
  after the check; no Fair-Play Factorio Pods remained.

Raw fixture logs and manual probe responses remain ignored local test artifacts.
They are deliberately not committed because they are environment-specific and
can identify test participants.

## Phase-1 boundary

Movement, mining, crafting, placement, inventory-capacity, and death/respawn
comparisons are not deferred Phase-0 defects. Each is a required manual
promotion probe that runs only after the corresponding typed Phase-1 primitive
has been implemented, tested for receipt/conservation, and deployed to a new
disposable fixture.

See [the implementation plan](implementation-plan.md) and
[the differential procedure](phase-0-differential-procedure.md).