//! The application-catalogue contract — the source of truth for "what is a
//! GUI app here".
//!
//! A caller is expected to refuse any executable the catalogue does not list,
//! so this is a security control and not just a convenience. Where the
//! catalogue comes from is per-OS (freedesktop `.desktop` entries on Linux,
//! `.app` bundles on macOS); *that it is the whitelist* is the contract.

use std::path::{Path, PathBuf};

/// One launchable entry.
///
/// Deliberately not `Serialize`: the wire shape of the catalogue belongs to
/// whichever adapter publishes it, so renaming a field here cannot silently
/// change what clients receive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopApp {
    /// Human-readable application name.
    pub name: String,
    /// The executable's basename — what a caller launches it by.
    pub exec: String,
}

/// The installed-GUI-apps catalogue, one implementation per OS.
pub trait AppCatalog: Send + Sync {
    /// Every installed GUI app, sorted by name (case-insensitively).
    fn apps(&self) -> Vec<DesktopApp>;

    /// The program to spawn for an `exec` from [`apps`](Self::apps), or
    /// `None` when the catalogue does not list it.
    ///
    /// Admission and lookup are the same call on purpose: a whitelist that
    /// answers "yes" and a resolver that then finds something else is the
    /// shape a confused-deputy bug takes. The catalogue is the security
    /// control, so it is what hands back the path.
    ///
    /// The two OSes disagree about what that path is — a bare basename
    /// resolved through `PATH` on Linux, the binary buried in an `.app`
    /// bundle on macOS — which is exactly why the caller must not guess.
    fn resolve(&self, exec: &str) -> Option<PathBuf>;
}

/// Every entry directly inside `dir` whose name ends in `.{extension}`.
///
/// Both catalogues are a directory of same-suffixed things — `.desktop`
/// files, `.app` bundles — and an unreadable directory is simply one that
/// contributes nothing, never an error: a machine without `/usr/games` or
/// `~/Applications` still has a catalogue.
pub fn entries_with_extension(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == extension))
        .collect()
}

/// Sort a catalogue the way [`AppCatalog::apps`] promises.
///
/// Here rather than in each backend so the ordering is enforced beside the
/// sentence that requires it, and a third catalogue inherits it.
pub fn sort_by_name(apps: &mut [DesktopApp]) {
    apps.sort_by_cached_key(|app| app.name.to_lowercase());
}
