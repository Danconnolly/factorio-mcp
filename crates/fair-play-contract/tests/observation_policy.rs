use fair_play_contract::{
    ActorId, Capability, CapabilityContract, EnabledMod, ForceId, ObservationLimits,
    PhaseZeroCapabilities, ProfileName, Provenance, SCHEMA_VERSION, SchedulingSemantics,
};
use serde_json::json;

fn contract_with_fixed_limits() -> CapabilityContract {
    CapabilityContract {
        schema_version: SCHEMA_VERSION.to_owned(),
        fair_play_policy_version: "0.1.0".to_owned(),
        bridge_build: "0.1.0".to_owned(),
        factorio_build: "2.0.0".to_owned(),
        enabled_mods: vec![EnabledMod::new("base", "2.0.0")],
        actor_id: ActorId::new("alfred"),
        force_id: ForceId::new("player"),
        profile_name: ProfileName::new("phase-zero-read-only"),
        observation_limits: ObservationLimits::new(32, 128, 4, 32 * 1024),
        scheduling: SchedulingSemantics::phase_zero(),
        capabilities: PhaseZeroCapabilities::exact(),
        supported_provenance: vec![Provenance::CharacterLocal, Provenance::ForceCharted],
        current_game_tick: 1,
    }
}

#[test]
fn observation_contract_publishes_all_fixed_bridge_limits() {
    let value = serde_json::to_value(contract_with_fixed_limits()).expect("contract serializes");
    assert_eq!(
        value["observation_limits"],
        json!({
            "max_radius": 32,
            "max_result_count": 128,
            "max_detail_fields": 4,
            "max_payload_bytes": 32 * 1024,
        })
    );
}

#[test]
fn observation_capabilities_remain_read_only_and_provenance_labeled() {
    let capabilities: Vec<_> = Capability::phase_zero_allow_list()
        .iter()
        .map(|capability| capability.as_str())
        .collect();
    assert_eq!(
        capabilities,
        [
            "get_capability_contract",
            "get_actor_status",
            "scan_local",
            "scan_charted",
            "get_entity",
            "get_action_record",
        ]
    );
    let contract = contract_with_fixed_limits();
    assert!(
        contract
            .supported_provenance
            .contains(&Provenance::CharacterLocal)
    );
    assert!(
        contract
            .supported_provenance
            .contains(&Provenance::ForceCharted)
    );
}
