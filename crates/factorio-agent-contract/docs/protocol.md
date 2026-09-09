# Phase-0 Fair-Play Contract Protocol

`factorio-agent-contract` is transport-independent. A bridge or MCP transport must first
return a `CapabilityContract` with `schema_version = "0.1.0"`; clients must reject a
version they do not support.

## Read-only capability surface

The Phase-0 allow-list is exact:

1. `get_capability_contract`
2. `get_actor_status`
3. `scan_local`
4. `scan_charted`
5. `get_entity`
6. `get_action_record`

No mutation, raw RCON, Lua, reset, teleport, item-grant, speed, or global-inspection
capability exists in this profile. Additions require a new fair-play policy version.

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

## Audit chain

`AuditRecord` has only `lifecycle` and `observation` variants. It is not a mutation
receipt format. Each append produces an envelope with a zero-based `sequence`, the
previous envelope digest, SHA-256 digest of canonical record JSON, caller-supplied
timestamp, and digest of the envelope link metadata. Canonical JSON recursively sorts
object keys and emits no insignificant whitespace. The chain hashes no credentials,
chat, inventory contents, player names, or unrelated-player data; those fields are not
represented by the Phase-0 audit model.

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
