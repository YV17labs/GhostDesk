//! Session-aware instrumentation for the server's own logs.
//!
//! The server already says what happened. What it could not say is what any
//! of it *cost*, because the three most useful facts about a tool call are
//! not local to the call:
//!
//! * **`gap_ms`** — the wait between one call returning and the next
//!   arriving. That is the model's thinking plus the API round trip: the
//!   larger half of the latency a user feels, and something the server can
//!   only know by remembering when it last answered.
//! * **friction** — the *same* action retried after changing nothing. A
//!   single dead click is ordinary and already logged where it happens; the
//!   repeat is the thing that points at a product fix.
//! * **the session summary** — totals that make two runs comparable, and
//!   that only exist once the run is over.
//!
//! Everything else a call could report is left to whoever already reports
//! it: `programs` logs its launches, `screen` its captures, `input` its
//! verdicts. This module adds a session, a sequence number and a stopwatch,
//! and gets out of the way.
//!
//! # Where it goes
//!
//! Into `tracing`, under the `features::telemetry` target, so it lands
//! wherever the operator's logs land — text in debug builds, JSON in release
//! — with no second store to collect, rotate or secure. Verbosity is
//! `<PREFIX>_LOG`'s job (`RUST_LOG` is the unprefixed fallback):
//!
//! | Level | Line | Volume |
//! |---|---|---|
//! | `warn` | a repeated futile action | rare, and always worth reading |
//! | `info` | one summary per session | one line per run |
//! | `debug` | one per tool call | tens per run |
//!
//! An OpenTelemetry layer (`nest-rs-opentelemetry`) exports all of it
//! unchanged, along with the `mcp.tool` span the MCP host opens around every
//! dispatch.

mod config;
mod mcp;
mod module;
mod service;
mod session;

/// Injected by every tool host, which runs its dispatch through it.
pub(crate) use mcp::CallJournal;
pub(crate) use mcp::TelemetryMcpModule;
/// Imported by the app's per-call MCP context module, which injects
/// [`TelemetryService`].
pub use module::TelemetryModule;
/// Injected by the app's per-call MCP context, which is glue over several
/// modules and therefore lives outside this crate.
pub use service::TelemetryService;
