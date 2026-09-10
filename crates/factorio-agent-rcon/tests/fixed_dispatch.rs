use std::{sync::Arc, time::Duration};

use factorio_agent_rcon::{
    BridgeCommand, BridgeErrorCode, BridgeResponse, EntityTarget, RconAdapter, RconConfig,
    RconError, RconTimeouts,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

const SECRET: &str = "do-not-log-this-password";

#[derive(Debug)]
struct Packet {
    id: i32,
    kind: i32,
    payload: String,
}

async fn receive(stream: &mut TcpStream) -> Packet {
    let mut length = [0_u8; 4];
    stream.read_exact(&mut length).await.unwrap();
    let body_length = usize::try_from(i32::from_le_bytes(length)).unwrap();
    let mut body = vec![0_u8; body_length];
    stream.read_exact(&mut body).await.unwrap();
    Packet {
        id: i32::from_le_bytes(body[0..4].try_into().unwrap()),
        kind: i32::from_le_bytes(body[4..8].try_into().unwrap()),
        payload: String::from_utf8(body[8..body_length - 2].to_vec()).unwrap(),
    }
}

async fn send(stream: &mut TcpStream, id: i32, kind: i32, payload: &str) {
    let body_length = i32::try_from(4 + 4 + payload.len() + 2).unwrap();
    stream.write_all(&body_length.to_le_bytes()).await.unwrap();
    stream.write_all(&id.to_le_bytes()).await.unwrap();
    stream.write_all(&kind.to_le_bytes()).await.unwrap();
    stream.write_all(payload.as_bytes()).await.unwrap();
    stream.write_all(&[0, 0]).await.unwrap();
    stream.flush().await.unwrap();
}

async fn start_server(responses: Vec<String>) -> (String, Arc<Mutex<Vec<Packet>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let received = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&received);
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let auth = receive(&mut stream).await;
        assert_eq!(auth.kind, 3);
        assert_eq!(auth.payload, SECRET);
        send(&mut stream, auth.id, 2, "").await;
        for response in responses {
            let command = receive(&mut stream).await;
            assert_eq!(command.kind, 2);
            captured.lock().await.push(command);
            let id = captured.lock().await.last().unwrap().id;
            send(&mut stream, id, 0, &response).await;
        }
    });
    (address, received)
}

fn contract_response() -> String {
    r#"{"ok":true,"result":{"schema_version":"0.4.0","fair_play_policy_version":"0.4.0","bridge_build":"0.4.0","factorio_build":"2.0.77","enabled_mods":[],"actor_id":"alfred","force_id":"player","profile_name":"phase-one-craft","observation_limits":{"max_radius":32,"max_result_count":128,"max_detail_fields":4,"max_payload_bytes":3072},"scheduling":{"execution":"tick_driven_character_input","observation_tick":"bridge_tick_snapshot","request_order":"one_active_action_serialized_at_next_game_tick"},"capabilities":["get_capability_contract","get_actor_status","scan_local","scan_charted","get_entity","get_action_record","walk_to","stop","mine","craft","get_action"],"supported_provenance":["character_local"],"current_game_tick":42}}"#.to_owned()
}

fn actor_response() -> String {
    r#"{"ok":true,"result":{"state":"ready","actor_id":"alfred","unit_number":9,"surface":{"index":1,"name":"nauvis"},"force":{"name":"player"},"position":{"x":0.0,"y":0.0},"health":250.0,"inventory":{"total":0,"items":[],"truncated":false},"tick":42,"bridge":{"build":"0.1.0","lifecycle_schema_version":1,"policy_version":"0.1.0"}}}"#.to_owned()
}

fn entity_response() -> String {
    r#"{"ok":true,"result":{"entity_id":"bridge-entity:9","name":"iron-chest","position":{"x":0.0,"y":0.0},"provenance":"permitted_direct_interaction","detail":[]}}"#.to_owned()
}

fn config(address: String) -> RconConfig {
    let directory = tempfile::tempdir().unwrap();
    let secret = directory.keep().join("rcon-password");
    std::fs::write(&secret, format!("{SECRET}\n")).unwrap();
    RconConfig::from_secret_file(address, secret)
        .unwrap()
        .with_timeouts(
            RconTimeouts::new(
                Duration::from_secs(1),
                Duration::from_secs(1),
                Duration::from_secs(1),
            )
            .unwrap(),
        )
}

#[tokio::test]
async fn enum_variants_use_the_same_fixed_bridge_interface() {
    let (address, received) = start_server(vec![
        contract_response(),
        contract_response(),
        actor_response(),
    ])
    .await;
    let adapter = RconAdapter::connect(config(address)).await.unwrap();

    assert!(matches!(
        adapter
            .call(BridgeCommand::GetCapabilityContract)
            .await
            .unwrap(),
        BridgeResponse::CapabilityContract(_)
    ));
    assert!(matches!(
        adapter.call(BridgeCommand::GetActorStatus).await.unwrap(),
        BridgeResponse::ActorStatus(_)
    ));

    let commands = received.lock().await;
    assert_eq!(commands.len(), 3);
    for command in commands.iter() {
        assert!(command.payload.starts_with("/c rcon.print(helpers.table_to_json(remote.call(\"factorio_agent_bridge\",\"query\",helpers.json_to_table("));
        assert!(command.payload.ends_with("))))"));
        assert!(!command.payload.contains(SECRET));
    }
    assert!(
        commands[0]
            .payload
            .contains(r#"{\"name\":\"get_capability_contract\"}"#)
    );
    assert!(
        commands[1]
            .payload
            .contains(r#"{\"name\":\"get_capability_contract\"}"#)
    );
    assert!(
        commands[2]
            .payload
            .contains(r#"{\"name\":\"get_actor_status\"}"#)
    );
}

#[tokio::test]
async fn bridge_version_mismatch_fails_closed() {
    let mismatched =
        contract_response().replace("\"bridge_build\":\"0.4.0\"", "\"bridge_build\":\"0.4.1\"");
    let (address, _) = start_server(vec![mismatched]).await;
    let Err(error) = RconAdapter::connect(config(address)).await else {
        panic!("mismatched bridge must be rejected");
    };
    assert!(matches!(error, RconError::VersionMismatch));
}

#[tokio::test]
async fn malformed_bridge_response_fails_closed() {
    let (address, _) = start_server(vec![contract_response(), "{not json}".to_owned()]).await;
    let adapter = RconAdapter::connect(config(address)).await.unwrap();
    let error = adapter
        .call(BridgeCommand::GetCapabilityContract)
        .await
        .unwrap_err();
    assert!(matches!(error, RconError::Bridge { .. }));
}

#[tokio::test]
async fn lua_and_rcon_looking_response_data_is_never_executed() {
    let mut response: serde_json::Value = serde_json::from_str(&actor_response()).unwrap();
    response["result"]["surface"]["name"] =
        serde_json::Value::String("\"; /c game.player.cheat_mode=true; --".to_owned());
    let response = serde_json::to_string(&response).unwrap();
    let (address, received) = start_server(vec![contract_response(), response]).await;
    let adapter = RconAdapter::connect(config(address)).await.unwrap();
    let response = adapter.call(BridgeCommand::GetActorStatus).await.unwrap();
    let BridgeResponse::ActorStatus(status) = response else {
        panic!("wrong response variant")
    };
    assert_eq!(
        status.surface.name,
        "\"; /c game.player.cheat_mode=true; --"
    );
    let command = &received.lock().await[0].payload;
    assert!(!command.contains("cheat_mode"));
}

#[test]
fn credential_errors_and_debug_output_are_redacted() {
    let directory = tempfile::tempdir().unwrap();
    let secret = directory.path().join("rcon-password");
    std::fs::write(&secret, SECRET).unwrap();
    let config = RconConfig::from_secret_file("127.0.0.1:27015", secret).unwrap();
    let rendered = format!(
        "{config:?} {}",
        RconError::configuration("invalid credentials")
    );
    assert!(!rendered.contains(SECRET));
}

#[tokio::test]
async fn request_data_is_escaped_inside_one_fixed_lua_literal() {
    let injection = "bridge-entity:9\")); /c game.player.cheat_mode=true; --";
    let (address, received) = start_server(vec![contract_response(), entity_response()]).await;
    let adapter = RconAdapter::connect(config(address)).await.unwrap();
    assert!(matches!(
        adapter
            .call(BridgeCommand::GetEntity {
                target: EntityTarget::IssuedId(injection.to_owned()),
            })
            .await
            .unwrap(),
        BridgeResponse::Entity(_)
    ));

    let payload = &received.lock().await[1].payload;
    let prefix = "/c rcon.print(helpers.table_to_json(remote.call(\"factorio_agent_bridge\",\"query\",helpers.json_to_table(";
    let literal = payload
        .strip_prefix(prefix)
        .unwrap()
        .strip_suffix("))))")
        .unwrap();
    assert_eq!(payload.matches("remote.call").count(), 1);
    // The payload may contain a second `/c` as hostile data; parsing the sole Lua string literal
    // proves that it remains data rather than executable RCON structure.
    let request_json: String = serde_json::from_str(literal).unwrap();
    let request: serde_json::Value = serde_json::from_str(&request_json).unwrap();
    assert_eq!(request["name"], "get_entity");
    assert_eq!(request["entity_id"], injection);
}

#[tokio::test]
async fn unknown_remote_error_code_is_not_exposed_to_callers() {
    let response = r#"{"ok":false,"error":{"code":"LEAK_THIS_REMOTE_CODE"}}"#.to_owned();
    let (address, _) = start_server(vec![contract_response(), response]).await;
    let adapter = RconAdapter::connect(config(address)).await.unwrap();
    let error = adapter
        .call(BridgeCommand::GetActorStatus)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        RconError::Bridge {
            code: BridgeErrorCode::UnknownRemoteError
        }
    ));
    assert!(!error.to_string().contains("LEAK_THIS_REMOTE_CODE"));
}
