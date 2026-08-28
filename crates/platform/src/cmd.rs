//! Async runner for the helper binaries a backend shells out to.
//!
//! Neutral because both backends need it and neither owns it: `swaymsg`,
//! `grim` and `wl-paste` on Linux, `screencapture`, `pbcopy` and `pbpaste` on
//! macOS. Never through a shell, so no argument is word-split or
//! glob-expanded — which is what makes it safe to hand a caller's region
//! geometry or clipboard text to one of them.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

/// Everything that can go wrong running a helper binary. Kept as a typed
/// error rather than `anyhow` because callers branch on the variants:
/// `sway::swaymsg` retries a dead socket on any failure, and the clipboard read
/// turns a non-zero exit into an empty string.
#[derive(Debug, thiserror::Error)]
pub enum CmdError {
    #[error("command timed out after {secs}s: {cmd}")]
    Timeout { cmd: String, secs: u64 },
    #[error("{0}")]
    Failed(String),
    #[error("could not spawn {cmd}: {source}")]
    Spawn {
        cmd: String,
        #[source]
        source: std::io::Error,
    },
}

/// Default ceiling for a helper that is expected to answer immediately.
///
/// Generous on purpose: every caller here is a local tool talking to a local
/// compositor or window server, so ten seconds is not a budget but a
/// deadlock's expiry date.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Run `argv` and return its stdout, trimmed. Never goes through a shell, so
/// no argument is ever word-split or glob-expanded.
pub async fn run(argv: &[&str], limit: Duration) -> Result<String, CmdError> {
    let raw = run_bytes(argv, limit).await?;
    Ok(String::from_utf8_lossy(&raw).trim().to_string())
}

/// Feed `input` to a helper on stdin and wait for it to exit.
///
/// Deliberately waits on the *child*, not on its pipes, which is what
/// separates this from [`run`]. `wl-copy` forks a daemon that keeps serving
/// the clipboard and inherits stdout/stderr, so those pipes never close and
/// `wait_with_output` would block for the session's lifetime; the parent
/// exiting right after the fork is what means "the clipboard is set".
pub async fn run_with_stdin(argv: &[&str], input: &[u8], limit: Duration) -> Result<(), CmdError> {
    let rendered = argv.join(" ");
    let mut child = Command::new(argv[0])
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|source| CmdError::Spawn {
            cmd: rendered.clone(),
            source,
        })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| CmdError::Failed(format!("{rendered} was spawned without a stdin pipe")))?;
    let written = async {
        stdin.write_all(input).await?;
        stdin.shutdown().await
    }
    .await;
    drop(stdin);
    written.map_err(|source| CmdError::Spawn {
        cmd: rendered.clone(),
        source,
    })?;

    match timeout(limit, child.wait()).await {
        Ok(status) => status.map_err(|source| CmdError::Spawn {
            cmd: rendered,
            source,
        })?,
        Err(_) => {
            return Err(CmdError::Timeout {
                cmd: rendered,
                secs: limit.as_secs(),
            });
        }
    };
    Ok(())
}

/// `run`, but handing back raw stdout — `grim` writes PNG to it.
pub async fn run_bytes(argv: &[&str], limit: Duration) -> Result<Vec<u8>, CmdError> {
    let rendered = argv.join(" ");
    let child = Command::new(argv[0])
        .args(&argv[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|source| CmdError::Spawn {
            cmd: rendered.clone(),
            source,
        })?;

    let output = match timeout(limit, child.wait_with_output()).await {
        Ok(result) => result.map_err(|source| CmdError::Spawn {
            cmd: rendered.clone(),
            source,
        })?,
        // `kill_on_drop` reaps the process when `child` is dropped on the way
        // out of this arm, so a timed-out helper never survives the call.
        Err(_) => {
            return Err(CmdError::Timeout {
                cmd: rendered,
                secs: limit.as_secs(),
            });
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CmdError::Failed(if stderr.is_empty() {
            format!("command failed with {}: {rendered}", output.status)
        } else {
            stderr
        }));
    }

    Ok(output.stdout)
}
