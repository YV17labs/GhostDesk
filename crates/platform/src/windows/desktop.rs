//! The Windows [`AppCatalog`] — Start Menu shortcuts.
//!
//! The Start Menu is what this operating system means by "the installed
//! applications": two directories of `.lnk` files, one for the machine and one
//! for the user, and what is in them is what the user sees when they press the
//! Windows key. It is the exact counterpart of freedesktop's `.desktop`
//! entries and of `/Applications`, and — like both — it is a whitelist rather
//! than a convenience, so a caller refuses anything it does not list.
//!
//! Two things differ from the other two catalogues. The tree is *nested*:
//! vendors install into a folder of their own, so the walk recurses where the
//! others read a flat directory. And a shortcut is not a path — it is a
//! structured file only the shell can read, so resolving one means COM. Both
//! are why this backend does its own walking instead of reaching for
//! [`desktop::entries_with_extension`](crate::desktop::entries_with_extension).
//!
//! What that leaves out is deliberate: a Store application's entry points at a
//! package identity rather than at an executable, so it resolves to nothing
//! and stays out of the catalogue. Launching one needs the shell to do it, and
//! a catalogue that listed what it cannot start would be a whitelist that
//! lies.

#![expect(
    unsafe_code,
    reason = "IShellLink is COM; each call carries its own SAFETY note"
)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ::windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize, IPersistFile, STGM_READ,
};
use ::windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use ::windows::core::{HSTRING, Interface, PCWSTR};

use crate::desktop::{self, AppCatalog, DesktopApp};

/// Where Windows keeps the Start Menu: one machine-wide tree and one per user.
///
/// Named by the variables that locate them rather than by literal paths — the
/// drive, the user's name and a redirected profile are all deployment facts
/// this crate has no business guessing.
const START_MENU_ROOTS: &[&str] = &["ProgramData", "APPDATA"];

/// The same path under each of them. One literal, because a subpath that
/// drifted on one row only would silently drop half the catalogue.
const START_MENU: &str = r"Microsoft\Windows\Start Menu\Programs";

/// How deep the walk goes.
///
/// The Start Menu is two levels in practice — a vendor folder and its
/// contents. The cap is what keeps a symlinked or pathological tree from
/// turning a catalogue read into an unbounded one.
const MAX_DEPTH: usize = 4;

/// The [`AppCatalog`] the host selector hands out on Windows.
pub struct StartMenuDesktop;

impl AppCatalog for StartMenuDesktop {
    fn apps(&self) -> Vec<DesktopApp> {
        let Some(shell) = Shell::open() else {
            return Vec::new();
        };

        // Named first, resolved second. The same application installed for the
        // machine *and* for the user is the ordinary case, and each copy costs
        // a shortcut load and a metadata query — spent, under the old order,
        // to build a row that was then thrown away. A name enters `resolved`
        // only once a copy has actually resolved, so a broken machine-wide
        // shortcut still lets the user's own copy through.
        let mut resolved: HashSet<String> = HashSet::new();
        let mut apps: Vec<DesktopApp> = start_menus()
            .iter()
            .flat_map(|dir| shortcuts_in(dir))
            .filter_map(|shortcut| {
                let name = shortcut_name(&shortcut)?;
                if resolved.contains(&name) {
                    return None;
                }
                shell.target(&shortcut)?;
                resolved.insert(name.clone());
                Some(DesktopApp {
                    exec: name.clone(),
                    name,
                })
            })
            .collect();

        desktop::sort_by_name(&mut apps);
        apps
    }

    fn resolve(&self, exec: &str) -> Option<PathBuf> {
        // Matched against the catalogue's own names rather than joined into a
        // directory: a name carrying `..` or a backslash would otherwise walk
        // straight out of the Start Menu this whitelist exists to enforce.
        let shortcut = start_menus()
            .iter()
            .flat_map(|dir| shortcuts_in(dir))
            .find(|shortcut| shortcut_name(shortcut).as_deref() == Some(exec))?;

        // COM is opened for this one file. `apps` resolves hundreds and holds
        // one shell object across all of them; a launch resolves exactly one.
        Shell::open()?.target(&shortcut)
    }
}

/// The Start Menu directories that exist on this machine.
fn start_menus() -> Vec<PathBuf> {
    START_MENU_ROOTS
        .iter()
        .filter_map(|variable| {
            std::env::var_os(variable).map(|root| Path::new(&root).join(START_MENU))
        })
        .collect()
}

/// Every `*.lnk` under `dir`, at any depth up to [`MAX_DEPTH`].
///
/// An unreadable directory contributes nothing rather than failing: a machine
/// with no per-user Start Menu still has a catalogue.
fn shortcuts_in(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(dir, 0, &mut found);
    found
}

fn walk(dir: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    if depth >= MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        // `file_type` reads the kind straight off the directory entry, where
        // `is_dir` would `stat` each one — a syscall per file across a menu of
        // several hundred.
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            walk(&path, depth + 1, found);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
        {
            found.push(path);
        }
    }
}

/// `…\Mozilla Firefox\Firefox.lnk` → `Firefox`.
///
/// The shortcut's own label, which is what the user reads in the Start Menu
/// and therefore what an agent asked to "open Firefox" will say — not the
/// executable buried behind it, which is as likely to be called
/// `firefox.exe` as `launcher.exe`.
fn shortcut_name(shortcut: &Path) -> Option<String> {
    Some(shortcut.file_stem()?.to_string_lossy().into_owned())
}

/// COM, opened for as long as one catalogue read takes.
///
/// Apartment-threaded because that is what the shell's own objects want, and
/// balanced by `Drop` because a blocking pool thread outlives the read and
/// would otherwise carry an apartment nobody asked for into the next task.
struct Shell {
    link: IShellLinkW,
    file: IPersistFile,
}

impl Shell {
    fn open() -> Option<Self> {
        // SAFETY: no pointers; balanced by `CoUninitialize` in `Drop`. A
        // second initialisation on the same thread reports `S_FALSE`, which is
        // a success and still owes its `CoUninitialize`.
        if unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_err() {
            return None;
        }

        // SAFETY: creates the shell's own link object in this process.
        let created =
            unsafe { CoCreateInstance::<_, IShellLinkW>(&ShellLink, None, CLSCTX_INPROC_SERVER) };
        let Ok(link) = created else {
            // SAFETY: balances the initialisation above, which succeeded.
            unsafe { CoUninitialize() };
            return None;
        };

        let Ok(file) = link.cast::<IPersistFile>() else {
            // SAFETY: as above.
            unsafe { CoUninitialize() };
            return None;
        };

        Some(Self { link, file })
    }

    /// The executable a shortcut points at, if it points at one that exists.
    ///
    /// One shell object, reloaded per file: `IPersistFile::Load` replaces
    /// whatever the link held, so a catalogue of several hundred shortcuts
    /// costs one object rather than several hundred.
    fn target(&self, shortcut: &Path) -> Option<PathBuf> {
        let path = HSTRING::from(shortcut.as_os_str());
        // SAFETY: the string outlives the call; `STGM_READ` opens the file
        // without writing to it.
        unsafe { self.file.Load(&path, STGM_READ) }.ok()?;

        let mut buffer = [0u16; 260];
        // SAFETY: the buffer bounds the write, and a null find-data pointer is
        // what the interface documents for "the file's metadata is not wanted".
        unsafe { self.link.GetPath(&mut buffer, std::ptr::null_mut(), 0) }.ok()?;

        // SAFETY: `GetPath` succeeded, so it wrote a NUL-terminated path into
        // the buffer, which is live here.
        let wide = PCWSTR(buffer.as_ptr());
        let target = PathBuf::from(String::from_utf16_lossy(unsafe { wide.as_wide() }));

        // A shortcut to a document, to a Store package, or to something since
        // uninstalled is not an application this catalogue can launch.
        let launchable = target
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
            && target.is_file();
        launchable.then_some(target)
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        // SAFETY: balances the `CoInitializeEx` in `open`, on the same thread.
        unsafe { CoUninitialize() };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_walk_finds_shortcuts_at_every_depth_and_ignores_everything_else() {
        let root = std::env::temp_dir().join(format!("gd-start-menu-{}", std::process::id()));
        let nested = root.join("Mozilla Firefox");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join("Notepad.lnk"), b"").unwrap();
        std::fs::write(nested.join("Firefox.lnk"), b"").unwrap();
        std::fs::write(nested.join("Uninstall.txt"), b"").unwrap();

        let mut names: Vec<String> = shortcuts_in(&root)
            .iter()
            .filter_map(|path| shortcut_name(path))
            .collect();
        names.sort();

        assert_eq!(names, vec!["Firefox".to_string(), "Notepad".to_string()]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_directory_that_is_not_there_contributes_nothing() {
        assert!(shortcuts_in(Path::new(r"Z:\no\such\start menu")).is_empty());
    }
}
