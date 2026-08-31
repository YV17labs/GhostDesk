//! Chord parsing — `Ctrl+Shift+T` → the virtual keys `SendInput` presses.
//!
//! The table agrees with the Linux one and disagrees with the macOS one, in
//! the single way that matters: `cmd` lands on the Windows key here, as it
//! lands on Super there, because Windows hangs its standard shortcuts off
//! Control. An agent told the desktop's primary modifier by
//! [`Conventions`](crate::input::Conventions) sends `ctrl+c` and copies.
//!
//! What the three tables may *not* disagree about is which tokens exist:
//! [`chord::MODIFIERS`] and [`chord::KEYS`] are the published set, every one
//! of them resolves here, and `chord::normalize` refuses everything else
//! before it reaches this file.

use anyhow::Result;

use ::windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_SHIFT,
};

use super::keycode;
use crate::chord;

/// Friendly modifier names → the key held down for them.
///
/// `option` sits beside `alt` because that is what the key is labelled on a
/// Mac keyboard, and `cmd`/`command` land on the Windows key so a chord an
/// agent learned on one desktop still reaches the modifier a Windows user
/// means by it.
const MODIFIERS: &[(&str, VIRTUAL_KEY)] = &[
    ("ctrl", VK_CONTROL),
    ("control", VK_CONTROL),
    ("alt", VK_MENU),
    ("option", VK_MENU),
    ("shift", VK_SHIFT),
    ("super", VK_LWIN),
    ("meta", VK_LWIN),
    ("win", VK_LWIN),
    ("cmd", VK_LWIN),
    ("command", VK_LWIN),
];

/// Resolve a chord to `(the modifiers held, the keys tapped under them)`.
///
/// A key whose character needs Shift on the active layout adds Shift to the
/// held set rather than to the tapped one — the modifier is what the layout
/// asked for, not a key the agent wants pressed.
pub(super) fn resolve(keys: &str) -> Result<(Vec<VIRTUAL_KEY>, Vec<VIRTUAL_KEY>)> {
    let mut held: Vec<VIRTUAL_KEY> = Vec::new();
    let mut plain = Vec::new();

    let mut hold = |modifier: VIRTUAL_KEY| {
        // `ctrl+control+c` and a shifted key under an explicit `shift` both
        // arrive here twice, and a modifier pressed twice is released once.
        if !held.contains(&modifier) {
            held.push(modifier);
        }
    };

    for token in chord::normalize(keys, &[])? {
        match MODIFIERS.iter().find(|(name, _)| *name == token) {
            Some((_, modifier)) => hold(*modifier),
            None => {
                let (key, needs_shift) = keycode::for_token(&token)?;
                if needs_shift {
                    hold(VK_SHIFT);
                }
                plain.push(key);
            }
        }
    }

    Ok((held, plain))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_the_modifiers_from_the_keys_they_hold() {
        let (held, keys) = resolve("ctrl+shift+t").unwrap();
        assert_eq!(held, vec![VK_CONTROL, VK_SHIFT]);
        assert_eq!(keys.len(), 1, "just `t`");
    }

    #[test]
    fn a_bare_key_holds_nothing() {
        let (held, keys) = resolve("enter").unwrap();
        assert!(held.is_empty());
        assert_eq!(keys, vec![super::keycode::for_token("enter").unwrap().0]);
    }

    #[test]
    fn a_modifier_named_twice_is_held_once() {
        // Held twice, released once, and every keystroke afterwards would
        // carry a Control the user never pressed.
        let (held, _) = resolve("ctrl+control+c").unwrap();
        assert_eq!(held, vec![VK_CONTROL]);
    }

    #[test]
    fn the_cross_desktop_super_names_all_land_on_the_windows_key() {
        for name in ["cmd", "command", "super", "meta", "win"] {
            let (held, _) = resolve(&format!("{name}+e")).unwrap();
            assert_eq!(held, vec![VK_LWIN], "{name}");
        }
    }

    #[test]
    fn control_stays_control() {
        let (held, _) = resolve("ctrl+c").unwrap();
        assert_eq!(
            held,
            vec![VK_CONTROL],
            "ctrl is this desktop's own primary modifier and must not be \
             promoted to anything",
        );
    }

    #[test]
    fn an_unknown_token_is_an_error_rather_than_a_silent_no_op() {
        assert!(resolve("ctrl+nonsense").is_err());
    }

    #[test]
    fn every_published_token_resolves() {
        // The published grammar is one contract over three backends, and this
        // is the half a test can prove: run on the target that owns the table.
        chord::every_published_token_resolves(resolve);
    }
}
