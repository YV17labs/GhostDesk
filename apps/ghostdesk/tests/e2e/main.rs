//! End-to-end suite — the tests that need live infrastructure. For GhostDesk
//! that means a real Wayland session: the supervisord stack (`nestrs run
//! stack`) with Sway up, since every feature service ultimately talks to the
//! compositor. `nestrs run test e2e` runs exactly this binary and `nestrs run
//! test unit` excludes it, so the fast suite never needs a desktop.
//!
//! Drive the composed app through `nest_rs::testing`'s `TestApp` the same way
//! an infrastructure-free suite in `tests/integration/main.rs` would, but
//! against the running compositor rather than a stubbed one.
//!
//! No tests yet — `nestrs run test e2e` passes an empty suite.
