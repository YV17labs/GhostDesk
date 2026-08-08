//! The Linux [`WindowManager`] — Sway IPC, with resilient socket discovery.
//!
//! The raw `get_tree` JSON never leaves this module: every node is folded
//! into a [`WindowInfo`] here, so the feature layer cannot grow a dependency
//! on Sway's tree shape.
//!
//! `$XDG_RUNTIME_DIR` is often a persistent volume in container deploys, so
//! dead `sway-ipc.<uid>.<pid>.sock` files from previous boots stick around
//! beside the live one. `$SWAYSOCK` cannot be trusted either (it is captured
//! when the MCP server starts and goes stale when Sway restarts), and neither
//! filename ordering nor mtime is authoritative. The only real test is "does
//! this socket answer Sway IPC *right now*?".
//!
//! 1. Glob `$XDG_RUNTIME_DIR/sway-ipc.*.sock`.
//! 2. Keep candidates whose embedded PID resolves to a live `sway` via
//!    `/proc/<pid>/comm`.
//! 3. Probe each survivor with `swaymsg -s <sock> -t get_version`.
//! 4. Cache the winner; on any later failure, drop the cache and re-discover
//!    once before giving up. That retry is what makes this self-heal across
//!    a Sway restart.
//!
//! Every call passes `-s <socket>` explicitly, so a stale `$SWAYSOCK`
//! inherited from the service wrapper can no longer mislead us.

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::cmd;
use crate::window::{WindowId, WindowInfo, WindowManager};

static CACHED_SOCK: Mutex<Option<String>> = Mutex::const_new(None);

fn runtime_dir() -> String {
    std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string())
}

/// `sway-ipc.<uid>.<pid>.sock` → `<pid>`.
fn pid_from_sock(name: &str) -> Option<u32> {
    let rest = name.strip_prefix("sway-ipc.")?.strip_suffix(".sock")?;
    let (_uid, pid) = rest.split_once('.')?;
    pid.parse().ok()
}

fn pid_is_sway(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|comm| comm.trim() == "sway")
        .unwrap_or(false)
}

/// Socket paths whose embedded PID points at a live sway.
fn candidate_socks() -> Vec<String> {
    let dir = runtime_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let pid = pid_from_sock(&name)?;
            pid_is_sway(pid).then(|| entry.path().to_string_lossy().into_owned())
        })
        .collect()
}

async fn probe(sock: &str) -> bool {
    cmd::run(
        &["swaymsg", "-s", sock, "-t", "get_version"],
        Duration::from_secs(2),
    )
    .await
    .is_ok()
}

/// Pick a live socket by actually probing each candidate.
async fn discover() -> Option<String> {
    let candidates = candidate_socks();
    if candidates.is_empty() {
        tracing::warn!(
            target: "platform::window",
            dir = %runtime_dir(),
            "no candidate socket with a live sway PID",
        );
        return None;
    }
    for sock in &candidates {
        if probe(sock).await {
            tracing::info!(target: "platform::window", socket = %sock, "using IPC socket");
            return Some(sock.clone());
        }
    }
    tracing::warn!(
        target: "platform::window",
        candidates = candidates.len(),
        "candidate sockets found but none answered get_version",
    );
    None
}

async fn resolve(force: bool) -> Option<String> {
    let mut cached = CACHED_SOCK.lock().await;
    if force || cached.is_none() {
        *cached = discover().await;
    }
    cached.clone()
}

/// Run `swaymsg` against the live socket, with one self-healing retry.
async fn swaymsg(args: &[&str], limit: Duration) -> anyhow::Result<String> {
    let sock = resolve(false)
        .await
        .ok_or_else(|| anyhow::anyhow!("sway: no live IPC socket found"))?;

    let invoke = |sock: String| async move {
        let mut argv = vec!["swaymsg", "-s", &sock];
        argv.extend_from_slice(args);
        cmd::run(&argv, limit).await
    };

    match invoke(sock).await {
        Ok(out) => Ok(out),
        Err(first) => {
            // Sway may have restarted under us, leaving the old socket dead.
            let sock = resolve(true)
                .await
                .ok_or_else(|| anyhow::Error::new(first))?;
            invoke(sock).await.map_err(Into::into)
        }
    }
}

/// The parsed Sway tree, or `None` on IPC/JSON failure.
async fn get_tree() -> Option<Value> {
    let raw = match swaymsg(&["-t", "get_tree"], cmd::DEFAULT_TIMEOUT).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::error!(target: "platform::window", error = %err, "get_tree failed");
            return None;
        }
    };
    match serde_json::from_str(&raw) {
        Ok(tree) => Some(tree),
        Err(err) => {
            tracing::error!(target: "platform::window", error = %err, "malformed get_tree JSON");
            None
        }
    }
}

/// Send a graceful close request to one Sway view by container id.
async fn kill_view(con_id: i64) -> anyhow::Result<()> {
    swaymsg(
        &[&format!("[con_id={con_id}]"), "kill"],
        cmd::DEFAULT_TIMEOUT,
    )
    .await
    .map(|_| ())
}

/// Every node in `tree` that has a `pid` — i.e. a real client surface.
///
/// Workspaces, outputs and the scratchpad have no pid, and layer-shell
/// clients (mako) are not part of the regular tree at all, so walking this
/// way never reaches the desktop's own infrastructure.
fn iter_views(tree: &Value) -> Vec<&Value> {
    let mut out = Vec::new();
    collect(tree, &mut out);
    out
}

fn collect<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    if node
        .get("pid")
        .and_then(Value::as_i64)
        .is_some_and(|p| p != 0)
    {
        out.push(node);
    }
    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = node.get(key).and_then(Value::as_array) {
            for child in children {
                collect(child, out);
            }
        }
    }
}

/// The X11 window class of a view, for the clients that have no `app_id`.
fn window_class(node: &Value) -> Option<&str> {
    node.get("window_properties")
        .and_then(|props| props.get("class"))
        .and_then(Value::as_str)
}

/// Fold one tree node into the neutral shape, or `None` for a node without a
/// container id — nothing upstream could address it.
fn window_info(node: &Value) -> Option<WindowInfo> {
    let con_id = node.get("id").and_then(Value::as_i64)?;
    Some(WindowInfo {
        id: WindowId::new(con_id, format!("con_id={con_id}")),
        // `app_id` for Wayland-native clients, else the X11 class for
        // XWayland ones.
        app: node
            .get("app_id")
            .and_then(Value::as_str)
            .or_else(|| window_class(node))
            .unwrap_or("?")
            .to_string(),
        title: node
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        pid: node.get("pid").and_then(Value::as_i64).unwrap_or_default(),
        focused: node
            .get("focused")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

/// The [`WindowManager`] the host selector hands out on Linux.
pub struct Sway;

#[async_trait]
impl WindowManager for Sway {
    async fn windows(&self) -> Result<Vec<WindowInfo>> {
        let tree = get_tree()
            .await
            .ok_or_else(|| anyhow::anyhow!("swaymsg get_tree failed (see server logs)"))?;
        Ok(iter_views(&tree)
            .into_iter()
            .filter_map(window_info)
            .collect())
    }

    async fn close(&self, window: &WindowId) -> Result<()> {
        kill_view(*window.payload::<i64>()?).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_the_pid_embedded_in_the_socket_name() {
        assert_eq!(pid_from_sock("sway-ipc.1000.4242.sock"), Some(4242));
        assert_eq!(pid_from_sock("sway-ipc.1000.sock"), None);
        assert_eq!(pid_from_sock("not-a-sway-socket"), None);
    }

    #[test]
    fn walks_nested_and_floating_nodes_but_skips_pidless_containers() {
        let tree = json!({
            "id": 1, "name": "root", "nodes": [
                {"id": 2, "name": "workspace", "nodes": [
                    {"id": 3, "pid": 111, "app_id": "firefox", "name": "Mozilla",
                     "focused": true},
                ], "floating_nodes": [
                    {"id": 4, "pid": 222, "name": "Dialog",
                     "window_properties": {"class": "Xdialog"}},
                ]},
            ],
        });

        let views: Vec<_> = iter_views(&tree)
            .into_iter()
            .filter_map(window_info)
            .collect();
        assert_eq!(views.len(), 2, "only nodes carrying a pid are views");

        let wayland = &views[0];
        assert_eq!(wayland.app, "firefox", "Wayland clients go by app_id");
        assert_eq!(wayland.title, "Mozilla");
        assert_eq!(wayland.pid, 111);
        assert!(wayland.focused);
        assert_eq!(*wayland.id.payload::<i64>().unwrap(), 3);

        let xwayland = &views[1];
        assert_eq!(xwayland.app, "Xdialog", "X11 clients fall back to class");
        assert_eq!(xwayland.title, "Dialog");
        assert!(!xwayland.focused);
    }

    #[test]
    fn an_empty_tree_yields_no_views() {
        assert!(iter_views(&json!({"id": 1, "nodes": []})).is_empty());
    }
}
