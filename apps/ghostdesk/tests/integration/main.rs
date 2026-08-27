//! What the composed app promises its clients, and what it refuses them:
//! one endpoint, the whole tool surface, the app's own identity, and a desk
//! that is closed to a caller without the secret. It boots the real graph and
//! does not *require* a desktop — the input backend reports itself unavailable
//! rather than aborting the boot — so it runs where `nestrs run test unit`
//! does, and on a CI runner with no compositor.

mod auth;
mod module;

/// The endpoint both suites address. Spelled out rather than read from
/// `nest_rs::mcp::DEFAULT_PATH`, now that no host spells it either: this is the
/// URL the README and SECURITY.md publish to clients, so the assertion has to
/// fail if the framework's default ever moves off it.
pub(crate) const PATH: &str = "/mcp";
