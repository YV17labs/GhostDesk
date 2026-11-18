use nest_rs::core::App;

use ghostdesk::GhostdeskModule;

#[nest_rs::main]
async fn main() -> anyhow::Result<()> {
    App::builder()
        .module::<GhostdeskModule>()
        .build()
        .await?
        .run()
        .await
}
