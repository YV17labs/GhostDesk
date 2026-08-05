//! GhostDesk's feature crate — one folder per domain, each exposing an
//! `#[injectable]` service, and a single `mcp/` adapter that publishes all of
//! them on one endpoint.
//!
//! The MCP spec namespaces tools *per endpoint*, and every shipped client
//! config points at one URL, so GhostDesk mounts exactly one `#[mcp]` host.
//! It plays the part a `#[controller]` plays over HTTP: thin, injecting the
//! domain services, translating between wire DTOs and domain calls.
//!
//! No domain here names an OS. Each service injects one of `platform`'s
//! backend contracts, and [`host`] is what binds them into the container.

pub mod apps;
pub mod clipboard;
pub mod host;
pub mod input;
pub mod mcp;
pub mod screen;
pub mod session;
pub mod telemetry;

#[cfg(test)]
mod testing;
