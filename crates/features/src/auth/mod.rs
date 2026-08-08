//! Auth domain — what the endpoint demands of a caller.
//!
//! Its own folder rather than a corner of an adapter because it is its own
//! concern, and the config already said so: the framework owns the
//! transport's namespace, so the token has always lived under `auth`. The
//! guard happens to be spelled as an `McpOperationGuard` — that is the seam
//! the transport offers, not the subject, which is why it sits in
//! [`mcp`](self::mcp) and the rule it enforces does not.

mod config;
mod error;
mod mcp;
mod module;
mod service;

pub use mcp::AuthMcpModule;
pub(crate) use module::AuthModule;
