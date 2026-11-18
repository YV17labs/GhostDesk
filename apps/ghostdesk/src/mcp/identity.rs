use nest_rs::mcp::McpIdentity;

use super::icons::icons;
use super::instructions::instructions;

/// What the endpoint says the server is, past what the binary already says.
///
/// The name, the version and the description are the binary's — read off this
/// crate's manifest by `#[nest_rs::main]` — so none of them is written here, and
/// none can drift from the release that serves it.
pub fn identity() -> McpIdentity {
    McpIdentity {
        title: Some("GhostDesk".into()),
        icons: Some(icons()),
        instructions: Some(instructions(&platform::host::conventions())),
        ..Default::default()
    }
}
