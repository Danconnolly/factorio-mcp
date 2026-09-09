# Factorio Agent Bridge Contract Protocol

`factorio-agent-contract` is transport-independent. A bridge or MCP transport must first
return a `CapabilityContract` with `schema_version = "0.2.0"`; clients must reject a
version they do not support.

## Phase-1 capability surface

The Phase-1 walk/stop allow-list is exact:

1. `get_capability_contract`
2. `get_actor_status`
3. `scan_local`
4. `scan_charted`
5. `get_entity`
6. `get_action_record`
7. `walk_to`
8. `stop`
9. `get_action`

Apart from `walk_to` and `stop`, no gameplay mutation is available. In particular,
no raw RCON, Lua, reset, teleport, item-grant, speed, mining, crafting, placement,
transfer, research, or global-inspection capability exists in this profile.
Additions require a new fair-play policy version.

## Contract fields and provenance

A discovery contract carries the schema and policy versions, bridge and Factorio
builds, enabled-mod manifest, bridge-owned actor and force IDs, profile name,
bridge-enforced observation limits, scheduling semantics, and current game tick.

Every entity, tile, or resource observation must carry one of these provenance
values: `character_local`, `force_charted`, `permitted_direct_interaction`, or
`force_statistics`. Each record's required `provenance` field is typed as this
enum; a transport must not substitute a broader provenance value.

`observation_limits` publishes fixed bridge-enforced values for `max_radius`,
`max_result_count`, `max_detail_fields`, and `max_payload_bytes`. A client can
request less but cannot increase any of them. Bounded scan responses carry their
requested and effective bounds, a single provenance value, sorted records,
`result_count`, `partial`, `truncated`, and `payload_bytes`.

The current `max_payload_bytes` is 3072 bytes. This deliberately leaves headroom
below Factorio's practical RCON console-response limit for the bridge response
envelope. A scan at the maximum radius must truncate deterministically rather
than producing an oversized transport response.

## Bridge observation policy

`scan_local` accepts only a radius and derives both the center and effective
bounds from the persisted virtual actor. It returns `character_local` records;
a caller cannot supply a remote center. A radius over the advertised limit is
`OUT_OF_POLICY`.

`scan_charted` accepts a bounded center and radius. Before any entity, tile, or
resource lookup, the bridge checks every intersecting chunk with the actor
force's chart state. Any uncharted chunk returns `UNCHARTED`; the bridge never
generates, charts, reveals, or substitutes a global search for that request.
Successful results are `force_charted`.

`get_entity` accepts either a bridge-issued entity ID from an earlier permitted
observation, or one local/charted position. A coordinate outside the local area
must first pass the same force-charted check. Arbitrary IDs and far uncharted
coordinates return `OUT_OF_POLICY` or `UNCHARTED`, rather than probing global
world state.

Each policy decision appends only the capability, requested/effective bounds,
provenance summary, result count, truncation state, decision, and game tick.
Audit records never contain observed world records or unrelated player data.

Errors have a stable `ErrorCode` for programs and a separate `detail` string for
people: `INVALID_ARGUMENT`, `ACTOR_UNAVAILABLE`, `OUT_OF_POLICY`, `UNCHARTED`,
`NOT_FOUND`, `BRIDGE_UNAVAILABLE`, `VERSION_MISMATCH`, and `INTERNAL`.

## Audit chain and action receipts

`AuditRecord` has `lifecycle`, `observation`, and durable action-receipt variants.
Every `walk_to` and `stop` request is retained by caller-supplied action ID before it
is acknowledged. A receipt includes the actor/action IDs, action sequence, request
fingerprint, policy and build context, lifecycle state/result, relevant ticks,
positions and health, explicit inventory delta, and affected entity IDs.

Each append produces an envelope with a zero-based `sequence`, the previous envelope
digest, SHA-256 digest of canonical record JSON, caller-supplied timestamp, and digest
of the envelope link metadata. Canonical JSON recursively sorts object keys and emits
no insignificant whitespace. The chain hashes no credentials, chat, player names, or
unrelated-player data.

Lifecycle records serialize exactly one of `accepted`, `running`, `succeeded`,
`rejected`, `cancelled`, or `failed`.

## Reproducible schemas

Checked-in fixtures are canonical JSON, not generated at build time:

- `docs/generated/capability-contract.schema.json`
- `docs/generated/audit-record.schema.json`
- `docs/generated/audit-envelope.schema.json`
- `docs/generated/entity-record.schema.json`
- `docs/generated/tile-record.schema.json`
- `docs/generated/resource-record.schema.json`

Regenerate after an intentional schema change:

```sh
cargo run -p factorio-agent-contract --example generate_schemas
cargo test -p factorio-agent-contract --test schema_contract -- --nocapture
```

The integration test compares generated schema values with these source fixtures and
also checks the exact allow-list, lifecycle serialization vocabulary, required typed
provenance on every observation-record type, required audit-envelope chain metadata,
and a fixed audit chain.
