//! `.desktop` entry parser — the source of truth for "what is a GUI app here".
//!
//! `app_launch` refuses any executable this module does not list, so the
//! catalogue is a security control, not just a convenience.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use configparser::ini::Ini;

const APPS_DIR: &str = "/usr/share/applications";

/// One launchable entry.
///
/// Deliberately not `Serialize`: the wire shape of the app catalogue belongs
/// to the MCP adapter (`features::mcp::dto::AppEntry`), so renaming a field
/// here cannot silently change what clients receive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopApp {
    /// Human-readable application name.
    pub name: String,
    /// The string to pass to `app_launch` — the executable's basename.
    pub exec: String,
}

/// Extract the executable basename from an `Exec=` field.
///
/// Strips field codes (`%u`, `%F`, …), a leading `env` wrapper, and
/// `KEY=value` assignments, so all of these yield `firefox`:
///
/// ```text
/// firefox %u
/// /usr/bin/firefox %U
/// env MOZ_ENABLE_WAYLAND=1 firefox
/// env MOZ_ENABLE_WAYLAND=1 /usr/bin/firefox %u
/// ```
pub fn parse_exec(field: &str) -> Option<String> {
    field.split_whitespace().find_map(|token| {
        if token.starts_with('%') || token == "env" || token.contains('=') {
            return None;
        }
        Path::new(token)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    })
}

/// True when the entry is a visible GUI app.
///
/// `Terminal=true` entries (vim, htop, …) are TUIs that need a tty —
/// launching one headless gives a process that exits instantly with no
/// window, so they do not belong in a GUI catalogue.
fn is_launchable(get: impl Fn(&str) -> Option<String>) -> bool {
    let flag = |key: &str| {
        get(key)
            .map(|raw| matches!(raw.trim().to_ascii_lowercase().as_str(), "1" | "true"))
            .unwrap_or(false)
    };
    get("Type").as_deref() == Some("Application")
        && !flag("NoDisplay")
        && !flag("Hidden")
        && !flag("Terminal")
}

fn entry_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "desktop"))
        .collect()
}

/// Parse one `.desktop` file into `(name, exec)`, or `None` when it is not a
/// launchable GUI entry.
fn read_entry(path: &Path) -> Option<DesktopApp> {
    // Case-sensitive: `Desktop Entry` keys are CamelCase (`NoDisplay`,
    // `Exec`), and the lowercasing parser would silently miss every one.
    let mut ini = Ini::new_cs();
    ini.load(path).ok()?;
    let get = |key: &str| ini.get("Desktop Entry", key);

    if !is_launchable(get) {
        return None;
    }

    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let exec = get("Exec")
        .and_then(|raw| parse_exec(&raw))
        .unwrap_or_else(|| stem.clone());

    Some(DesktopApp {
        name: get("Name").unwrap_or(stem),
        exec,
    })
}

/// Every installed GUI app, sorted by name (case-insensitively).
pub fn desktop_apps() -> Vec<DesktopApp> {
    apps_in(Path::new(APPS_DIR))
}

/// The set of executable basenames `app_launch` will accept.
pub fn known_executables() -> BTreeSet<String> {
    desktop_apps().into_iter().map(|app| app.exec).collect()
}

fn apps_in(dir: &Path) -> Vec<DesktopApp> {
    let mut apps: Vec<DesktopApp> = entry_files(dir)
        .iter()
        .filter_map(|path| read_entry(path))
        .collect();
    apps.sort_by_key(|app| app.name.to_lowercase());
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_field_codes_env_wrapper_and_assignments() {
        assert_eq!(parse_exec("firefox %u").as_deref(), Some("firefox"));
        assert_eq!(
            parse_exec("/usr/bin/firefox %U").as_deref(),
            Some("firefox")
        );
        assert_eq!(
            parse_exec("env MOZ_ENABLE_WAYLAND=1 firefox").as_deref(),
            Some("firefox")
        );
        assert_eq!(
            parse_exec("env MOZ_ENABLE_WAYLAND=1 /usr/bin/firefox %u").as_deref(),
            Some("firefox")
        );
        assert_eq!(parse_exec("").as_deref(), None);
    }

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn catalogues_only_visible_gui_entries() {
        let dir = std::env::temp_dir().join(format!("gd-desktop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        write(
            &dir,
            "firefox.desktop",
            "[Desktop Entry]\nType=Application\nName=Firefox\nExec=firefox %u\n",
        );
        write(
            &dir,
            "vim.desktop",
            "[Desktop Entry]\nType=Application\nName=Vim\nExec=vim\nTerminal=true\n",
        );
        write(
            &dir,
            "hidden.desktop",
            "[Desktop Entry]\nType=Application\nName=Hidden\nExec=hidden\nNoDisplay=true\n",
        );
        write(
            &dir,
            "link.desktop",
            "[Desktop Entry]\nType=Link\nName=Link\nURL=https://example.com\n",
        );

        let apps = apps_in(&dir);
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(
            apps,
            vec![DesktopApp {
                name: "Firefox".into(),
                exec: "firefox".into(),
            }],
            "TUI, hidden and non-Application entries stay out of the catalogue",
        );
    }
}
