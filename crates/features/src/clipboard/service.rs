//! Clipboard access through `wl-copy` / `wl-paste`.

use std::process::Stdio;
use std::time::Duration;

use nest_rs::core::injectable;
use platform::cmd;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[injectable]
#[derive(Default)]
pub struct ClipboardService;

impl ClipboardService {
    /// The current clipboard as text.
    ///
    /// Empty when the clipboard is empty or holds non-text content —
    /// `wl-paste` exits non-zero for both, and neither is an error worth
    /// spending an agent turn on.
    pub async fn get(&self) -> String {
        cmd::run(&["wl-paste", "--no-newline"], cmd::DEFAULT_TIMEOUT)
            .await
            .unwrap_or_default()
    }

    /// Write text to the clipboard.
    pub async fn set(&self, text: &str) -> anyhow::Result<String> {
        // `wl-copy` reads stdin, then forks a daemon that keeps serving the
        // content to other apps. We wait for the *parent* to exit — that
        // happens right after the fork and means the clipboard is set. We
        // must not wait on stdout/stderr closing: those pipes are inherited
        // by the daemon child and would never close.
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("wl-copy gave us no stdin"))?;
        stdin.write_all(text.as_bytes()).await?;
        stdin.shutdown().await?;
        drop(stdin);

        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .map_err(|_| anyhow::anyhow!("wl-copy did not exit within 5s"))??;

        Ok(format!(
            "Clipboard set ({} characters)",
            text.chars().count()
        ))
    }
}
