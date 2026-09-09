use fair_play_contract::{
    ActionId, ActionState, ActorId, ActorLifecycleState, ActorStatus, AuditChain, AuditRecord,
    BridgeVersions, Capability, CapabilityContract, EnabledMod, ForceId, ForceIdentity,
    InventoryItemSummary, InventorySummary, ObservationLimits, ObservationRecord, Position,
    ProfileName, Provenance, SCHEMA_VERSION, SchedulingSemantics, SurfaceIdentity,
    actor_status_schema, audit_envelope_schema, audit_schema, canonical_json, capability_schema,
    entity_record_schema, resource_record_schema, tile_record_schema,
};
use serde_json::Value;
use std::fs;

fn contract() -> CapabilityContract {
    CapabilityContract {
        schema_version: SCHEMA_VERSION.to_owned(),
        fair_play_policy_version: "phase-0.1".to_owned(),
        bridge_build: "bridge-0.1.0".to_owned(),
        factorio_build: "2.0.0".to_owned(),
        enabled_mods: vec![EnabledMod::new("base", "2.0.0")],
        actor_id: ActorId::new("actor:phase-zero"),
        force_id: ForceId::new("force:player"),
        profile_name: ProfileName::new("phase-zero"),
        observation_limits: ObservationLimits::new(32, 128, 4, 32 * 1024),
        scheduling: SchedulingSemantics::phase_zero(),
        supported_provenance: vec![
            Provenance::CharacterLocal,
            Provenance::ForceCharted,
            Provenance::PermittedDirectInteraction,
            Provenance::ForceStatistics,
        ],
        current_game_tick: 42,
    }
}

#[test]
fn phase_zero_capability_allow_list_is_exact_and_excludes_mutation() {
    let names: Vec<_> = Capability::phase_zero_allow_list()
        .iter()
        .map(|capability| capability.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "get_capability_contract",
            "get_actor_status",
            "scan_local",
            "scan_charted",
            "get_entity",
            "get_action_record",
        ]
    );
    for prohibited in [
        "rcon",
        "lua",
        "teleport",
        "grant_items",
        "set_game_speed",
        "reset_world",
        "mine_resource",
        "build_entity",
    ] {
        assert!(
            !names.contains(&prohibited),
            "prohibited tool: {prohibited}"
        );
    }
}

#[test]
fn capability_schema_and_fixture_require_the_contract_and_provenance() {
    let schema = capability_schema();
    let schema_text = canonical_json(&schema).expect("schema canonicalizes");
    let fixture = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/generated/capability-contract.schema.json"
    ))
    .expect("checked-in schema fixture");
    assert_eq!(fixture.trim(), schema_text);
    for required in [
        "schema_version",
        "fair_play_policy_version",
        "bridge_build",
        "factorio_build",
        "enabled_mods",
        "actor_id",
        "force_id",
        "profile_name",
        "observation_limits",
        "scheduling",
        "current_game_tick",
    ] {
        assert!(
            schema_text.contains(required),
            "missing contract field: {required}"
        );
    }
    assert!(schema_text.contains("character_local"));
    assert!(schema_text.contains("force_charted"));
    assert!(schema_text.contains("permitted_direct_interaction"));
    assert!(schema_text.contains("force_statistics"));
    for fixed_limit in [
        "max_radius",
        "max_result_count",
        "max_detail_fields",
        "max_payload_bytes",
    ] {
        assert!(
            schema_text.contains(fixed_limit),
            "missing fixed limit: {fixed_limit}"
        );
    }
}

#[test]
fn audit_schema_is_checked_in_and_future_mutation_receipts_are_not_phase_zero_records() {
    let schema = audit_schema();
    let schema_text = canonical_json(&schema).expect("schema canonicalizes");
    let fixture = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/generated/audit-record.schema.json"
    ))
    .expect("checked-in schema fixture");
    assert_eq!(fixture.trim(), schema_text);
    assert!(schema_text.contains("lifecycle"));
    assert!(schema_text.contains("observation"));
    assert!(!schema_text.contains("mutation_receipt"));
    let schema: Value = serde_json::to_value(schema).expect("schema serializes");
    assert_eq!(
        schema["$defs"]["ActionState"]["enum"],
        serde_json::json!([
            "accepted",
            "running",
            "succeeded",
            "rejected",
            "cancelled",
            "failed"
        ])
    );
}

#[test]
fn every_observation_record_requires_typed_provenance() {
    for (record_name, fixture_name, schema) in [
        ("EntityRecord", "entity-record", entity_record_schema()),
        ("TileRecord", "tile-record", tile_record_schema()),
        (
            "ResourceRecord",
            "resource-record",
            resource_record_schema(),
        ),
    ] {
        let schema_value = serde_json::to_value(&schema).expect("schema serializes");
        let required = schema_value["required"]
            .as_array()
            .expect("required fields");
        assert!(
            required.iter().any(|field| field == "provenance"),
            "{record_name} must require provenance"
        );
        assert_eq!(
            schema_value["properties"]["provenance"]["$ref"], "#/$defs/Provenance",
            "{record_name} provenance must reference the typed Provenance enum"
        );

        let fixture = fs::read_to_string(format!(
            "{}/docs/generated/{fixture_name}.schema.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("checked-in source schema fixture");
        assert_eq!(
            fixture.trim(),
            canonical_json(&schema).expect("schema canonicalizes")
        );
    }
}

#[test]
fn audit_envelope_schema_is_checked_in_with_chain_metadata() {
    let schema = audit_envelope_schema();
    let schema_text = canonical_json(&schema).expect("schema canonicalizes");
    let fixture = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/generated/audit-envelope.schema.json"
    ))
    .expect("checked-in audit envelope source schema fixture");
    assert_eq!(fixture.trim(), schema_text);

    let schema: Value = serde_json::to_value(schema).expect("schema serializes");
    let required = schema["required"].as_array().expect("required fields");
    for field in [
        "sequence",
        "previous_digest",
        "record_digest",
        "timestamp",
        "envelope_digest",
    ] {
        assert!(
            required
                .iter()
                .any(|required_field| required_field == field),
            "AuditEnvelope must require {field}"
        );
    }
}

#[test]
fn action_identifiers_and_all_six_lifecycle_states_are_stable() {
    let id = ActionId::new("action:0001");
    assert_eq!(id.as_str(), "action:0001");
    assert_eq!(
        ActionState::ALL.map(|state| serde_json::to_value(state).expect("state serializes")),
        [
            Value::String("accepted".to_owned()),
            Value::String("running".to_owned()),
            Value::String("succeeded".to_owned()),
            Value::String("rejected".to_owned()),
            Value::String("cancelled".to_owned()),
            Value::String("failed".to_owned())
        ]
    );
}

#[test]
fn audit_hash_chain_is_deterministic_for_fixed_records() {
    let records = [
        AuditRecord::Lifecycle {
            action_id: ActionId::new("action:0001"),
            state: ActionState::Accepted,
            game_tick: 41,
        },
        AuditRecord::Observation(ObservationRecord {
            capability: Capability::ScanLocal,
            game_tick: 42,
            provenance: Provenance::CharacterLocal,
            result_count: 3,
            truncated: false,
        }),
    ];
    let mut first = AuditChain::new();
    let envelopes: Vec<_> = records
        .iter()
        .enumerate()
        .map(|(index, record)| {
            first
                .append(record, 1_700_000_000 + index as u64)
                .expect("append")
        })
        .collect();
    let mut second = AuditChain::new();
    let repeated: Vec<_> = records
        .iter()
        .enumerate()
        .map(|(index, record)| {
            second
                .append(record, 1_700_000_000 + index as u64)
                .expect("append")
        })
        .collect();
    assert_eq!(envelopes, repeated);
    assert_eq!(envelopes[0].sequence, 0);
    assert_eq!(envelopes[0].previous_digest, AuditChain::GENESIS_DIGEST);
    assert_eq!(envelopes[1].sequence, 1);
    assert_eq!(envelopes[1].previous_digest, envelopes[0].envelope_digest);
    AuditChain::verify(&envelopes).expect("fixed chain verifies");
}

#[test]
fn contract_is_machine_readable_and_complete() {
    let value = serde_json::to_value(contract()).expect("serializes");
    assert_eq!(value["schema_version"], SCHEMA_VERSION);
    assert_eq!(value["observation_limits"]["max_radius"], 32);
    assert_eq!(value["observation_limits"]["max_result_count"], 128);
    assert_eq!(value["observation_limits"]["max_detail_fields"], 4);
    assert_eq!(value["observation_limits"]["max_payload_bytes"], 32 * 1024);
}

#[test]
fn actor_status_is_typed_read_only_and_checked_in() {
    let status = ActorStatus {
        state: ActorLifecycleState::Ready,
        actor_id: ActorId::new("alfred"),
        unit_number: 77,
        surface: SurfaceIdentity {
            index: 1,
            name: "nauvis".to_owned(),
        },
        force: ForceIdentity {
            name: "player".to_owned(),
        },
        position: Position { x: 3.5, y: -4.0 },
        health: 250.0,
        inventory: InventorySummary {
            total: 1,
            items: vec![InventoryItemSummary {
                name: "iron-plate".to_owned(),
                count: 1,
            }],
            truncated: false,
        },
        tick: 123,
        bridge: BridgeVersions {
            build: "0.1.0".to_owned(),
            lifecycle_schema_version: 1,
            policy_version: "0.1.0".to_owned(),
        },
    };
    let value = serde_json::to_value(status).expect("status serializes");
    for field in ["state", "position", "health", "inventory", "tick", "bridge"] {
        assert!(value.get(field).is_some(), "actor status requires {field}");
    }
    let schema = actor_status_schema();
    let fixture = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/generated/actor-status.schema.json"
    ))
    .expect("checked-in actor status schema fixture");
    assert_eq!(
        fixture.trim(),
        canonical_json(&schema).expect("schema canonicalizes")
    );
}
