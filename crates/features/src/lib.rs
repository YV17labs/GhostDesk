//! GhostDesk's domains, each wrapping one seam of the `platform` substrate.
//!
//! No domain here names an OS: the ones that touch the desktop reach it
//! through one of `platform`'s five backend contracts, and [`auth`] and
//! [`telemetry`] — which are about the server rather than the desktop —
//! inject none.
//!
//! Every domain publishes its own `/mcp` host, and the framework merges them
//! onto one endpoint. That is why there is no cross-domain adapter here and
//! why the app imports edges rather than domains: [`host`] is the substrate
//! they all sit on, and the endpoint's own identity is declared by the app,
//! which is the only layer that can see the whole surface.
//!
//! Two ports are exported anyway — [`idle`] and [`telemetry`] — because the
//! app's per-call MCP context injects their services, and the access graph
//! demands that whoever provides a consumer import the modules that provide
//! its dependencies.

pub mod auth;
pub mod clipboard;
pub(crate) mod host;
pub mod idle;
pub mod input;
pub mod programs;
pub mod screen;
pub mod telemetry;

#[cfg(test)]
mod testing;
