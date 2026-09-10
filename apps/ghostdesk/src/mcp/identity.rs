use nest_rs::mcp::McpIdentity;

use super::icons::icons;
use super::instructions::instructions;

pub fn identity() -> McpIdentity {
    McpIdentity::new("ghostdesk", env!("CARGO_PKG_VERSION"))
        .title("GhostDesk")
        .description("MCP server to control a virtual desktop")
        .icons(icons())
        .instructions(instructions(&platform::host::conventions()))
}
