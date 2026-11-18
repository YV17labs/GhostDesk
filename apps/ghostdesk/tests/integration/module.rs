use ghostdesk::GhostdeskModule;
use nest_rs::core::Discovery;
use nest_rs::mcp::{McpHostMeta, endpoint_identity};
use nest_rs::testing::TestApp;

use crate::PATH;

const TOOLS: &[&str] = &[
    "app_launch",
    "app_list",
    "app_running",
    "app_status",
    "clipboard_get",
    "clipboard_set",
    "key_press",
    "key_type",
    "mouse_click",
    "mouse_double_click",
    "mouse_drag",
    "mouse_move",
    "mouse_scroll",
    "screen_shot",
];

/// Booted as the binary boots it, under the package's own name and version —
/// what `#[nest_rs::main]` publishes for the binary, `TestApp` publishes for
/// the suite.
async fn app() -> TestApp {
    TestApp::builder()
        .module::<GhostdeskModule>()
        .build()
        .await
        .expect("the composition root assembles")
}

/// Every MCP host the app mounts on `PATH`.
fn hosts(app: &TestApp) -> Vec<std::sync::Arc<McpHostMeta>> {
    Discovery::new(app.container())
        .meta::<McpHostMeta>()
        .into_iter()
        .map(|discovered| discovered.meta)
        .filter(|meta| meta.path() == PATH)
        .collect()
}

#[tokio::test]
async fn every_edge_lands_on_one_endpoint() {
    let app = app().await;
    let hosts: Vec<&str> = hosts(&app).iter().map(|meta| meta.host()).collect();

    assert_eq!(hosts.len(), 4, "hosts on {PATH}: {hosts:?}");
    for expected in ["ScreenTool", "InputTool", "ProgramsTool", "ClipboardTool"] {
        assert!(
            hosts.contains(&expected),
            "{expected} missing from {hosts:?}"
        );
    }
}

#[tokio::test]
async fn the_hosts_publish_the_whole_tool_surface_and_no_name_twice() {
    let app = app().await;

    let mut names: Vec<String> = hosts(&app)
        .iter()
        .flat_map(|meta| meta.declared_tools())
        .map(|tool| tool.name.to_string())
        .collect();
    names.sort();

    assert_eq!(names, TOOLS, "the published tool surface");
}

#[tokio::test]
async fn the_app_names_the_endpoint_rather_than_leaving_it_to_a_host() {
    let app = app().await;
    let identity = endpoint_identity(app.container(), PATH);

    let info = identity
        .implementation()
        .expect("the endpoint reports an implementation");
    assert_eq!(info.name, "ghostdesk");
    assert_eq!(
        info.version,
        env!("CARGO_PKG_VERSION"),
        "the app's version, never the SDK's",
    );
    assert_eq!(info.title.as_deref(), Some("GhostDesk"));

    let brief = identity
        .instructions()
        .expect("the session brief is declared, not left to be joined from four hosts");
    assert!(brief.contains("Copy is"), "the brief names the shortcuts");
}
