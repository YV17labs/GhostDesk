//! End-to-end suite — the tests that need a live desktop, since every feature
//! service ultimately talks to one. On Linux that means a Wayland session with
//! Sway up, which is what the supervisord stack brings; on macOS it means the
//! session you are already logged into, because `host::input()` and its four
//! siblings bind Quartz and the Accessibility API directly, with no stack in
//! between.
//!
//! Boot the composed app the way `tests/integration/` does — `App::builder()`
//! over `GhostdeskModule` — but against that live desktop rather than a graph
//! whose services never reach one.
