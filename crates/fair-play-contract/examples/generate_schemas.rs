use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = root.join("docs/generated");
    fs::create_dir_all(&output).expect("create schema output directory");
    fs::write(
        output.join("capability-contract.schema.json"),
        fair_play_contract::canonical_json(&fair_play_contract::capability_schema())
            .expect("canonical capability schema"),
    )
    .expect("write capability schema");
    fs::write(
        output.join("actor-status.schema.json"),
        fair_play_contract::canonical_json(&fair_play_contract::actor_status_schema())
            .expect("canonical actor status schema"),
    )
    .expect("write actor status schema");
    fs::write(
        output.join("audit-record.schema.json"),
        fair_play_contract::canonical_json(&fair_play_contract::audit_schema())
            .expect("canonical audit schema"),
    )
    .expect("write audit schema");
    fs::write(
        output.join("audit-envelope.schema.json"),
        fair_play_contract::canonical_json(&fair_play_contract::audit_envelope_schema())
            .expect("canonical audit envelope schema"),
    )
    .expect("write audit envelope schema");
    fs::write(
        output.join("entity-record.schema.json"),
        fair_play_contract::canonical_json(&fair_play_contract::entity_record_schema())
            .expect("canonical entity record schema"),
    )
    .expect("write entity record schema");
    fs::write(
        output.join("tile-record.schema.json"),
        fair_play_contract::canonical_json(&fair_play_contract::tile_record_schema())
            .expect("canonical tile record schema"),
    )
    .expect("write tile record schema");
    fs::write(
        output.join("resource-record.schema.json"),
        fair_play_contract::canonical_json(&fair_play_contract::resource_record_schema())
            .expect("canonical resource record schema"),
    )
    .expect("write resource record schema");
}
