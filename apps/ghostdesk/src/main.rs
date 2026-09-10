use nest_rs::config::Environment;
use nest_rs::core::App;
use nest_rs::guards::{AppBuilderGuardsExt, guard};

use features::auth::AuthnGuard;
use ghostdesk::{CallTrailGuard, GhostdeskModule};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _environment = Environment::init();

    App::builder()
        .use_guards_global([guard::<AuthnGuard>(), guard::<CallTrailGuard>()])
        .module::<GhostdeskModule>()
        .build()
        .await?
        .run()
        .await
}
