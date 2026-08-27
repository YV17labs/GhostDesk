//! The macOS [`AppCatalog`] — `.app` bundles.
//!
//! A bundle is a directory, and the part GhostDesk needs is a fixed path
//! inside it: `Foo.app/Contents/MacOS/<binary>`. That is read straight off
//! the filesystem rather than through `NSWorkspace`, for the same reason the
//! clipboard shells out — two directory reads against a documented layout
//! beat pulling AppKit and a run loop into a headless server.
//!
//! `Info.plist` is deliberately not parsed. The only two fields that would
//! come from it are the display name, which is the bundle's own directory
//! stem in every case a user would recognise, and `CFBundleExecutable`,
//! which by construction names the single file in `Contents/MacOS`. Reading
//! the directory answers both without a plist parser (and without a binary
//! plist decoder, which half of them need).

use std::path::{Path, PathBuf};

use crate::desktop::{self, AppCatalog, DesktopApp};

/// Where macOS installs GUI applications. `~/Applications` is included
/// because a per-user install is still an app the agent can be asked to
/// drive; everything else on the system is out of the catalogue, and a
/// caller refuses what the catalogue does not list.
const BUNDLE_DIRS: &[&str] = &[
    "/Applications",
    "/Applications/Utilities",
    "/System/Applications",
    "/System/Applications/Utilities",
];

/// The [`AppCatalog`] the host selector hands out on macOS.
pub struct AppBundles;

impl AppCatalog for AppBundles {
    fn apps(&self) -> Vec<DesktopApp> {
        let mut apps: Vec<DesktopApp> = bundle_dirs()
            .iter()
            .flat_map(|dir| bundles_in(dir))
            .filter_map(|bundle| read_bundle(&bundle))
            .collect();
        desktop::sort_by_name(&mut apps);
        apps.dedup_by(|a, b| a.exec == b.exec);
        apps
    }

    fn resolve(&self, exec: &str) -> Option<PathBuf> {
        // Re-derive the path from the catalogue rather than joining `exec`
        // into a directory: a name carrying `..` or a slash would otherwise
        // walk straight out of the bundle directories this whitelist exists
        // to enforce.
        bundle_dirs()
            .iter()
            .flat_map(|dir| bundles_in(dir))
            .find(|bundle| bundle_name(bundle).as_deref() == Some(exec))
            .and_then(|bundle| executable_in(&bundle))
    }
}

/// The search path: the system directories plus the calling user's own.
fn bundle_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = BUNDLE_DIRS.iter().map(PathBuf::from).collect();
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(Path::new(&home).join("Applications"));
    }
    dirs
}

/// Every `*.app` directly inside `dir`.
fn bundles_in(dir: &Path) -> Vec<PathBuf> {
    desktop::entries_with_extension(dir, "app")
}

/// `…/Firefox.app` → `Firefox`.
fn bundle_name(bundle: &Path) -> Option<String> {
    Some(bundle.file_stem()?.to_string_lossy().into_owned())
}

/// The binary inside a bundle: the single regular file in `Contents/MacOS`.
///
/// A bundle with several (a helper alongside the app) is skipped rather than
/// guessed at — launching the wrong one gives the agent a process that exits
/// with no window and no explanation.
fn executable_in(bundle: &Path) -> Option<PathBuf> {
    // `file_type()` reads the kind straight off the directory entry, where
    // `is_file()` would `stat` each one — a syscall per bundle across a
    // catalogue of a few hundred.
    let mut binaries = std::fs::read_dir(bundle.join("Contents/MacOS"))
        .ok()?
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path());

    let first = binaries.next()?;
    binaries.next().is_none().then_some(first)
}

/// A bundle that has a launchable binary → its catalogue entry.
fn read_bundle(bundle: &Path) -> Option<DesktopApp> {
    let name = bundle_name(bundle)?;
    executable_in(bundle)?;
    Some(DesktopApp {
        // The bundle stem is both the name a user says and the handle a
        // caller launches by, so nobody has to learn that the binary inside is
        // called something else (`Firefox` → `firefox`).
        exec: name.clone(),
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(root: &Path, name: &str, binaries: &[&str]) {
        let macos = root.join(format!("{name}.app/Contents/MacOS"));
        std::fs::create_dir_all(&macos).unwrap();
        for binary in binaries {
            std::fs::write(macos.join(binary), b"#!/bin/sh\n").unwrap();
        }
    }

    #[test]
    fn a_bundle_is_named_by_its_stem_and_resolves_to_the_binary_inside() {
        let root = std::env::temp_dir().join(format!("gd-bundles-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        bundle(&root, "Firefox", &["firefox"]);

        let found = bundles_in(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(bundle_name(&found[0]).as_deref(), Some("Firefox"));
        assert_eq!(
            read_bundle(&found[0]),
            Some(DesktopApp {
                name: "Firefox".into(),
                exec: "Firefox".into(),
            }),
            "the agent says `Firefox`, not the binary name buried inside",
        );
        assert!(
            executable_in(&found[0])
                .unwrap()
                .ends_with("Contents/MacOS/firefox"),
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_ambiguous_or_empty_bundle_stays_out_of_the_catalogue() {
        let root = std::env::temp_dir().join(format!("gd-bundles-amb-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        bundle(&root, "Ambiguous", &["app", "helper"]);
        bundle(&root, "Empty", &[]);

        for found in bundles_in(&root) {
            assert_eq!(
                read_bundle(&found),
                None,
                "a bundle we cannot launch unambiguously is not offered at all",
            );
        }

        std::fs::remove_dir_all(&root).ok();
    }
}
