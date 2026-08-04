use ghostdesk::GhostdeskModule;
use nest_rs::core::App;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    App::builder()
        .module::<GhostdeskModule>()
        .build()
        .await?
        .run()
        .await
}
