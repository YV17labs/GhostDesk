use ghostdesk::GhostdeskModule;
use nest_rs::core::App;
use nest_rs::mcp::{endpoint_identity, hosts_on};

/// Spelled out rather than read from `nest_rs::mcp::DEFAULT_PATH`, now that no
/// host spells it either: this is the URL the README and SECURITY.md publish to
/// clients, so the assertion has to fail if the framework's default ever moves
/// off it. Taking the constant would only assert the hosts are wherever the
/// framework put them, which is not the promise being kept.
const PATH: &str = "/mcp";

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

async fn app() -> App {
    App::builder()
        .module::<GhostdeskModule>()
        .build()
        .await
        .expect("the composition root assembles")
}

#[tokio::test]
async fn every_edge_lands_on_one_endpoint() {
    let app = app().await;
    let hosts: Vec<&str> = hosts_on(app.container(), PATH)
        .iter()
        .map(|meta| meta.host())
        .collect();

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

    let mut names: Vec<String> = hosts_on(app.container(), PATH)
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
        .expect("the app declares an identity for the endpoint it composes");
    assert_eq!(info.name, "ghostdesk");
    assert_eq!(
        info.version,
        env!("CARGO_PKG_VERSION"),
        "the app's version, never the SDK's",
    );

    let brief = identity
        .instructions()
        .expect("the session brief is declared, not left to be joined from four hosts");
    assert!(brief.contains("Copy is"), "the brief names the shortcuts");
}
