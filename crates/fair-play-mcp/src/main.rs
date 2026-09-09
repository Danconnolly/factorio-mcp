//! Stdio MCP server entry point for the Phase-0 read-only profile.

#![forbid(unsafe_code)]

fn main() {
    // Keep this binary inert until the shared contract and closed RCON dispatch
    // are implemented. In particular, it must not expose arbitrary Lua/RCON.
    println!(
        "fair-play-mcp Phase 0 skeleton (contract schema {})",
        fair_play_contract::SCHEMA_VERSION
    );
}
