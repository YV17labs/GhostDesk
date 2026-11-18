//! What the composed app promises its clients, and what it refuses them:
//! one endpoint, the whole tool surface, the app's own identity, and a desk
//! that is closed to a caller without the secret. It boots the real graph and
//! does not *require* a desktop — the input backend reports itself unavailable
//! rather than aborting the boot — so it runs where `nestrs run test unit`
//! does, and on a CI runner with no compositor.

use nest_rs::testing::TestApp;

use features::auth::AuthConfig;
use ghostdesk::GhostdeskModule;

mod auth;
mod module;

/// The endpoint both suites address. Spelled out rather than read from
/// `nest_rs::mcp::DEFAULT_PATH`, now that no host spells it either: this is the
/// URL the README and SECURITY.md publish to clients, so the assertion has to
/// fail if the framework's default ever moves off it.
pub(crate) const PATH: &str = "/mcp";

/// The composed app, under the token a suite chooses, booted as the binary
/// boots it — under the package's own name and version, which `TestApp`
/// publishes as `#[nest_rs::main]` does.
///
/// The token is seeded rather than read from the environment: the variables
/// are process-wide and every test in this binary shares them, so a suite that
/// sets one decides the posture of the tests running beside it.
pub(crate) async fn desk(token: Option<&str>) -> TestApp {
    TestApp::builder()
        .module::<GhostdeskModule>()
        .provide(AuthConfig {
            token: token.map(Into::into),
        })
        .build()
        .await
        .expect("the composition root assembles")
}
