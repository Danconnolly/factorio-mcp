use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn production_lua(name: &str) -> String {
    fs::read_to_string(
        workspace_root()
            .join("mods/factorio-fair-play-bridge")
            .join(name),
    )
    .expect("read production Lua")
}

#[test]
fn lifecycle_uses_persisted_unit_number_and_provenance_not_player_selection() {
    let actor = production_lua("actor.lua");
    for forbidden in [
        "game.players",
        "game.get_player",
        "game.player",
        "[1]",
        "commands.add_command",
    ] {
        assert!(
            !actor.contains(forbidden),
            "forbidden lifecycle path: {forbidden}"
        );
    }
    for required in [
        "storage.factorio_fair_play_bridge",
        "game.get_entity_by_unit_number(record.unit_number)",
        "character.name_tag == record.provenance_marker",
        "surface.find_non_colliding_position",
        "force.get_spawn_position(surface)",
        "starting_inventory_policy = \"empty\"",
    ] {
        assert!(
            actor.contains(required),
            "missing lifecycle guard: {required}"
        );
    }
}

#[test]
fn remote_protocol_is_narrow_read_only_and_rejects_future_requests() {
    let protocol = production_lua("protocol.lua");
    let read_only = production_lua("read_only.lua");
    for forbidden in ["remote.call", "loadstring", "load(", "commands.add_command"] {
        assert!(
            !protocol.contains(forbidden),
            "forbidden protocol path: {forbidden}"
        );
    }
    assert!(
        protocol
            .contains("remote.add_interface(config.BRIDGE_INTERFACE, { query = protocol.query })")
    );
    assert!(protocol.contains("unknown or future mutation request"));
    for capability in [
        "scan_local",
        "scan_charted",
        "get_entity",
        "get_action_record",
    ] {
        assert!(protocol.contains(capability), "protocol lacks {capability}");
    }
    assert!(protocol.contains("request arguments do not match the read-only capability"));
    for required in ["state", "position", "health", "inventory", "tick", "bridge"] {
        assert!(
            read_only.contains(required),
            "actor status lacks {required}"
        );
    }
    for required in [
        "local function enabled_mods()",
        "table.sort(mods",
        "enabled_mods = enabled_mods()",
    ] {
        assert!(
            read_only.contains(required),
            "capability contract lacks deterministic enabled-mod output: {required}"
        );
    }
}

#[test]
fn lifecycle_hooks_revalidate_the_stored_actor_and_status_has_no_gameplay_writes() {
    let control = production_lua("control.lua");
    let read_only = production_lua("read_only.lua");
    for required in [
        "script.on_init(function()",
        "script.on_configuration_changed(function()",
        "actor.initialise()",
        "script.on_load(function()",
        "actor.on_load()",
        "script.on_nth_tick(1, actor.validate_after_load)",
    ] {
        assert!(
            control.contains(required),
            "missing lifecycle hook: {required}"
        );
    }
    for forbidden in [
        "create_entity",
        "destroy(",
        "teleport",
        "insert(",
        "remove_item",
    ] {
        assert!(
            !read_only.contains(forbidden),
            "read-only status must not mutate gameplay: {forbidden}"
        );
    }
}

#[test]
fn startup_actor_setting_is_explicit_and_documented_for_disposable_tests() {
    let settings = production_lua("settings.lua");
    let config = production_lua("config.lua");
    let fixture = fs::read_to_string(workspace_root().join("tests/mod-settings/mod-settings.cfg"))
        .expect("read documented fixture");
    assert!(settings.contains("setting_type = \"startup\""));
    assert!(config.contains("DEFAULT_ACTOR_ID = \"alfred\""));
    assert_eq!(
        fixture.trim(),
        "[mod-setting-name]\nfactorio-fair-play-actor-id=alfred"
    );
}

#[test]
fn observation_policy_is_bridge_enforced_bounded_and_chart_aware() {
    let policy = production_lua("scripts/observation_policy.lua");
    let read_only = production_lua("read_only.lua");
    let audit = production_lua("audit.lua");

    for required in [
        "MAX_OBSERVATION_RADIUS",
        "MAX_OBSERVATION_RESULTS",
        "MAX_OBSERVATION_DETAIL_FIELDS",
        "MAX_OBSERVATION_PAYLOAD_BYTES",
        "requested radius exceeds bridge observation policy",
        "force.is_chunk_charted(surface, { x = chunk_x, y = chunk_y })",
        "#helpers.table_to_json(result) > config.MAX_OBSERVATION_PAYLOAD_BYTES",
        "table.sort(result.entities, by_name_then_position)",
        "table.sort(result.resources, by_name_then_position)",
        "table.sort(result.tiles, by_name_then_position)",
        "bridge-entity:",
    ] {
        assert!(
            policy.contains(required),
            "missing observation policy guard: {required}"
        );
    }
    for forbidden in [
        "game.force_chart",
        "surface.request_to_generate_chunks",
        "game.players",
    ] {
        assert!(
            !policy.contains(forbidden),
            "observation policy must not reveal world state: {forbidden}"
        );
    }
    for required in [
        "local audit = require(\"audit\")",
        "policy.local_bounds(character, request.radius)",
        "policy.require_charted(character.force, character.surface, bounds)",
        "surface.find_tiles_filtered({ area = area })",
        "entity_id was not issued by a permitted bridge observation",
        "provenance = \"character_local\"",
        "provenance = \"force_charted\"",
        "observation_limits = {",
    ] {
        assert!(
            read_only.contains(required),
            "read-only bridge lacks {required}"
        );
    }
    for required in [
        "requested_bounds",
        "effective_bounds",
        "provenance",
        "result_count",
    ] {
        assert!(
            audit.contains(required),
            "audit lacks policy summary {required}"
        );
    }
    assert!(!audit.contains("inventory"));
}
