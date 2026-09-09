//! Stdio entry point for the Phase-0 read-only MCP server.

#![forbid(unsafe_code)]

use std::{env, path::PathBuf, process::ExitCode};

use fair_play_mcp::{AuditSink, PhaseZeroServer};
use fair_play_rcon::{RconAdapter, RconConfig};
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> ExitCode {
    if env::args_os()
        .nth(1)
        .is_some_and(|argument| argument == "--help" || argument == "-h")
    {
        println!(
            "fair-play-mcp\n\nStdio MCP server exposing only the Phase-0 read-only profile.\n\nEnvironment:\n  FAIR_PLAY_AUDIT_LOG                 Append-only JSONL audit log path\n  FAIR_PLAY_RCON_ADDRESS              Factorio RCON address\n  FAIR_PLAY_RCON_PASSWORD[_FILE]      Exactly one RCON credential source\n  FAIR_PLAY_RCON_EXPECTED_BRIDGE_BUILD Expected bridge build (optional)"
        );
        return ExitCode::SUCCESS;
    }
    if env::args_os().nth(1).is_some() {
        eprintln!("unsupported argument; use --help");
        return ExitCode::from(2);
    }

    let audit_path = env::var_os("FAIR_PLAY_AUDIT_LOG")
        .map_or_else(|| PathBuf::from("fair-play-mcp-audit.jsonl"), PathBuf::from);
    let audit = match AuditSink::open(audit_path) {
        Ok(audit) => audit,
        Err(error) => {
            eprintln!("audit verification failed: {:?}", error.code);
            return ExitCode::from(1);
        }
    };
    let config = match RconConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("RCON configuration failed: {error}");
            return ExitCode::from(1);
        }
    };
    let bridge = match RconAdapter::connect(config).await {
        Ok(bridge) => bridge,
        Err(error) => {
            eprintln!("bridge connection failed: {error}");
            return ExitCode::from(1);
        }
    };
    let contract = match bridge
        .call(fair_play_rcon::BridgeCommand::GetCapabilityContract)
        .await
    {
        Ok(fair_play_rcon::BridgeResponse::CapabilityContract(contract)) => contract,
        Ok(_) => {
            eprintln!("bridge contract response failed validation");
            return ExitCode::from(1);
        }
        Err(error) => {
            eprintln!("bridge contract read failed: {error}");
            return ExitCode::from(1);
        }
    };
    match PhaseZeroServer::new(bridge, audit, contract)
        .serve(rmcp::transport::stdio())
        .await
    {
        Ok(service) => {
            let _ = service.waiting().await;
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("MCP server failed: {error}");
            ExitCode::from(1)
        }
    }
}
