//! The chord grammar, and the opaque chord a backend resolves it to.
//!
//! One published vocabulary, one resolution per desktop. [`MODIFIERS`] and
//! [`KEYS`] are the tokens a caller may send on *any* desktop — the tool
//! description quotes them — while what each token becomes is the backend's
//! business: `cmd` is Super on Linux, the Windows key on Windows and Command
//! on macOS, and merging those tables would
//! reintroduce exactly the bug [`Conventions`](crate::input::Conventions)
//! exists to prevent.
//!
//! The split matters because only half of it used to exist. `normalize` was
//! shared and the token tables were not, so `option` resolved on one desktop
//! and failed on the other under a single published description — a chord the
//! agent was told to send, refused by half the fleet.
//!
//! Both directions are enforced, and by two different mechanisms because they
//! are two different failures. *Published ⊆ served* is a test —
//! `every_published_token_resolves`, run from each backend's own module, so it
//! runs on the target that owns the table. *Served ⊆ published* cannot be a
//! test, because a backend's tables inevitably carry internal names that are
//! reachable by anyone who guesses them: `leftctrl` and `rightmeta` are how the
//! Linux keymap spells its modifiers, and both resolved there and nowhere else.
//! So [`normalize`] refuses any multi-character token the lists below do not
//! name, before any backend's table is consulted — one gate, every desktop,
//! nothing to remember.

use std::any::Any;

use anyhow::{Result, bail};

/// Modifier tokens every backend resolves.
///
/// Ten names for four modifiers, because an agent writes the chord its training
/// spelled: `control` and `ctrl`, `option` beside `alt` for a Mac keyboard's
/// own label, and `super`/`meta`/`win`/`cmd`/`command` all landing on whatever
/// this desktop hangs its shortcuts off.
pub const MODIFIERS: &[&str] = &[
    "ctrl", "control", "alt", "option", "shift", "super", "meta", "win", "cmd", "command",
];

/// Non-printable key tokens every backend resolves.
///
/// Printable keys are not listed: a single character resolves through the
/// backend's own layout handling, and enumerating Unicode here would be a
/// table that could only ever be incomplete.
pub const KEYS: &[&str] = &[
    "return",
    "enter",
    "escape",
    "esc",
    "backspace",
    "delete",
    "tab",
    "space",
    "home",
    "end",
    "pageup",
    "page_up",
    "pagedown",
    "page_down",
    "left",
    "right",
    "up",
    "down",
    "f1",
    "f2",
    "f3",
    "f4",
    "f5",
    "f6",
    "f7",
    "f8",
    "f9",
    "f10",
    "f11",
    "f12",
];

/// A chord resolved by the backend that will press it.
///
/// The payload is opaque on purpose: on Wayland a chord is an XKB modifier
/// mask plus keysyms, on macOS it is `(CGEventFlags, CGKeyCode)` — no neutral
/// encoding covers both without lying to one of them. Only the backend that
/// produced a `Chord` can consume it, and only one backend ever exists per
/// process.
pub struct Chord(Box<dyn Any + Send>);

impl Chord {
    pub fn new(payload: impl Any + Send) -> Self {
        Self(Box::new(payload))
    }

    /// Recover the payload.
    ///
    /// The error means a backend was handed a chord it did not resolve — a
    /// programming error, not a runtime condition. The diagnostic lives here
    /// rather than in each backend so every implementation reports it the
    /// same way without having to reword it.
    pub fn take<T: Any>(self) -> Result<T> {
        match self.0.downcast::<T>() {
            Ok(payload) => Ok(*payload),
            Err(_) => bail!("chord was resolved by a different input backend"),
        }
    }
}

/// Published spellings of one key, folded before a backend sees either.
///
/// [`KEYS`] publishes both halves of each pair, so choosing between them is
/// the grammar's own business and not a per-OS table: a backend carrying it
/// would carry the same four rows the other one does — the exact duplication
/// this file exists to close. A backend's own aliases still win, since they
/// map a published name onto an internal one.
pub const KEY_ALIASES: &[(&str, &str)] = &[
    ("return", "enter"),
    ("escape", "esc"),
    ("page_up", "pageup"),
    ("page_down", "pagedown"),
];

/// Is this a name the grammar publishes?
fn published(token: &str) -> bool {
    MODIFIERS.contains(&token) || KEYS.contains(&token)
}

/// Split a `+`-separated chord into normalised tokens, applying `aliases`.
///
/// One definition rather than a convention three backends happen to agree on:
/// the grammar is published to callers, so it cannot be per-OS. A token is
/// either a single character — a key on every keyboard, resolved through the
/// backend's own layout handling — or a *name*, and a name is either published
/// or it does not exist. Refusing the rest here is what keeps a backend's
/// internal spelling from becoming an accidental second grammar that only one
/// desktop answers to.
///
/// `aliases` then maps a published name to whatever this backend calls it, so
/// the values returned are the backend's own and need no further checking.
pub fn normalize(keys: &str, aliases: &[(&str, &str)]) -> Result<Vec<String>> {
    keys.split('+')
        .filter(|token| !token.trim().is_empty())
        .map(|token| {
            let key = token.trim().to_lowercase();
            if key.chars().count() > 1 && !published(&key) {
                bail!("unknown key name: {key:?}");
            }
            Ok(aliases
                .iter()
                .chain(KEY_ALIASES)
                .find_map(|(from, to)| (*from == key).then(|| (*to).to_string()))
                .unwrap_or(key))
        })
        .collect()
}

/// Assert that a backend's `resolve` serves every published token.
///
/// Called from each backend's own test module, so the check runs on the target
/// that owns the table rather than on whichever one the developer happens to
/// be building. Modifiers are probed in a real chord (`ctrl+a`) because a
/// modifier alone resolves to an empty key list on every backend and would
/// pass without ever reaching the table.
#[cfg(test)]
pub(crate) fn every_published_token_resolves<T, E: std::fmt::Display>(
    resolve: impl Fn(&str) -> std::result::Result<T, E>,
) {
    for modifier in MODIFIERS {
        if let Err(err) = resolve(&format!("{modifier}+a")) {
            panic!("this backend refuses the published modifier {modifier:?}: {err}");
        }
    }
    for key in KEYS {
        if let Err(err) = resolve(key) {
            panic!("this backend refuses the published key {key:?}: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALIASES: &[(&str, &str)] = &[("return", "enter")];

    #[test]
    fn splits_a_chord_dropping_blanks_and_applying_aliases() {
        assert_eq!(
            normalize("Ctrl+Shift+Return", ALIASES).unwrap(),
            vec!["ctrl", "shift", "enter"],
        );
        assert_eq!(normalize("ctrl++a", ALIASES).unwrap(), vec!["ctrl", "a"]);
        assert_eq!(normalize(" ", ALIASES).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn a_name_the_grammar_does_not_publish_is_refused_for_every_backend() {
        // `leftctrl` and `rightmeta` are how the Linux keymap spells its own
        // modifiers. Both resolved there and on no other desktop, under one
        // published description — which is the divergence this gate closes
        // without a backend having to prune its table.
        for unpublished in ["leftctrl", "rightmeta", "capslock", "insert", "del", "fn"] {
            assert!(
                normalize(unpublished, ALIASES).is_err(),
                "{unpublished:?} is served by a backend but published by none",
            );
        }
    }

    #[test]
    fn a_single_character_needs_no_publishing() {
        // Enumerating Unicode is the one thing the lists cannot do, so a lone
        // character is always a key and the backend's layout handling decides.
        for key in ["a", "7", "é", "€"] {
            assert!(normalize(key, ALIASES).is_ok(), "{key:?}");
        }
    }

    #[test]
    fn a_chord_handed_to_the_wrong_backend_says_so() {
        let chord = Chord::new(42u32);
        assert!(
            chord
                .take::<String>()
                .unwrap_err()
                .to_string()
                .contains("different input backend")
        );
    }

    #[test]
    fn the_published_vocabulary_carries_no_duplicates() {
        // A duplicate would make `every_published_token_resolves` probe the
        // same token twice and still leave a gap somewhere else — the list is
        // the contract, so it has to be readable as a set.
        for (index, token) in MODIFIERS.iter().enumerate() {
            assert!(
                !MODIFIERS[index + 1..].contains(token),
                "{token:?} is published twice as a modifier",
            );
        }
        for (index, token) in KEYS.iter().enumerate() {
            assert!(
                !KEYS[index + 1..].contains(token),
                "{token:?} is published twice as a key",
            );
        }
    }
}
