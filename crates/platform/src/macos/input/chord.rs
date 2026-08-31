//! Chord parsing — `Cmd+Shift+T` → the flags and keycodes a `CGEvent` needs.
//!
//! This is the one table that genuinely disagrees with its Linux counterpart,
//! and the disagreement is not cosmetic. `cmd` maps to Super on Linux and to
//! Command here, and Command is the modifier macOS puts every standard
//! shortcut behind: copy is `cmd+c`, not `ctrl+c`. An agent that sends the
//! Linux chord on macOS gets a control character in a text field instead of a
//! clipboard, silently. The published tool instructions say which to send;
//! this table is what makes the answer true.
//!
//! What the two tables may *not* disagree about is which tokens exist:
//! [`chord::MODIFIERS`] and [`chord::KEYS`] are the published set, every one of
//! them resolves here, and `chord::normalize` refuses everything else before it
//! reaches this file — so a name only one desktop knows cannot become a chord
//! only one desktop answers.

use anyhow::Result;
use objc2_core_graphics::{CGEventFlags, CGKeyCode};

use super::keycode;
use crate::chord;

/// Friendly modifier names → the flag they raise.
///
/// `option` is spelled out beside `alt` because that is what the key is
/// labelled on a Mac keyboard, and `super`/`meta`/`win` land on Command so a
/// chord written for any desktop still reaches the modifier a Mac user means.
const MODIFIERS: &[(&str, CGEventFlags)] = &[
    ("ctrl", CGEventFlags::MaskControl),
    ("control", CGEventFlags::MaskControl),
    ("alt", CGEventFlags::MaskAlternate),
    ("option", CGEventFlags::MaskAlternate),
    ("shift", CGEventFlags::MaskShift),
    ("cmd", CGEventFlags::MaskCommand),
    ("command", CGEventFlags::MaskCommand),
    ("super", CGEventFlags::MaskCommand),
    ("meta", CGEventFlags::MaskCommand),
    ("win", CGEventFlags::MaskCommand),
];

/// Resolve a chord to `(modifier flags, the keys held under them)`.
///
/// Modifiers are accumulated into the flag set stamped on each key event,
/// which is how Quartz expects a chord to be expressed — separate
/// `FlagsChanged` events would leave the modifier latched if anything went
/// wrong between the two posts.
pub(super) fn resolve(keys: &str) -> Result<(CGEventFlags, Vec<CGKeyCode>)> {
    let mut flags = CGEventFlags::empty();
    let mut plain = Vec::new();

    for token in chord::normalize(keys, &[])? {
        match MODIFIERS.iter().find(|(name, _)| *name == token) {
            Some((_, flag)) => flags |= *flag,
            None => plain.push(keycode::for_token(&token)?),
        }
    }

    Ok((flags, plain))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_friendly_key_names_to_the_internal_ones() {
        assert_eq!(
            chord::normalize("Return+ ESCAPE +Page_Up", &[]).unwrap(),
            vec!["enter", "esc", "pageup"],
        );
    }

    #[test]
    fn separates_the_modifier_flags_from_the_keys_they_hold() {
        let (flags, keys) = resolve("cmd+shift+t").unwrap();
        assert_eq!(flags, CGEventFlags::MaskCommand | CGEventFlags::MaskShift);
        assert_eq!(keys, vec![17], "just `t`");
    }

    #[test]
    fn a_bare_key_carries_no_flags() {
        let (flags, keys) = resolve("enter").unwrap();
        assert_eq!(flags, CGEventFlags::empty());
        assert_eq!(keys, vec![36]);
    }

    #[test]
    fn the_cross_desktop_super_names_all_land_on_command() {
        // A chord written for any desktop has to reach the modifier a Mac
        // user actually means, or every shortcut an agent knows is wrong.
        for name in ["cmd", "command", "super", "meta", "win"] {
            let (flags, _) = resolve(&format!("{name}+c")).unwrap();
            assert_eq!(flags, CGEventFlags::MaskCommand, "{name}");
        }
    }

    #[test]
    fn control_stays_control() {
        let (flags, _) = resolve("ctrl+c").unwrap();
        assert_eq!(
            flags,
            CGEventFlags::MaskControl,
            "ctrl must not be quietly promoted to cmd — an agent asking for a \
             terminal interrupt means Control",
        );
    }

    #[test]
    fn an_unknown_token_is_an_error_rather_than_a_silent_no_op() {
        assert!(resolve("cmd+nonsense").is_err());
    }

    #[test]
    fn every_published_token_resolves() {
        // The published grammar is one contract over three backends. Before this
        // ran, `del` and `fn` resolved here and nowhere else, under a single
        // tool description that promised neither.
        chord::every_published_token_resolves(resolve);
    }
}
