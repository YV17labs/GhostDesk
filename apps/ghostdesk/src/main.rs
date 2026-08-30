use ghostdesk::GhostdeskModule;
use nest_rs::config::Environment;
use nest_rs::core::App;
use nest_rs::guards::AppBuilderGuardsExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _environment = Environment::init();
    App::builder()
        .use_guards_global(GhostdeskModule::guards())
        .module::<GhostdeskModule>()
        .build()
        .await?
        .run()
        .await
}
