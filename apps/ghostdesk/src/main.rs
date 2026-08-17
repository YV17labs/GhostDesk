use ghostdesk::GhostdeskModule;
use nest_rs::config::Environment;
use nest_rs::core::App;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _environment = Environment::init();
    App::builder()
        .module::<GhostdeskModule>()
        .build()
        .await?
        .run()
        .await
}
