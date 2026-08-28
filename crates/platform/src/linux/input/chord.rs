//! Chord parsing — `Ctrl+Shift+Tab` → the keysyms and modifier mask the
//! virtual keyboard needs.
//!
//! The table is backend-private, the vocabulary it must cover is not: every
//! token in [`chord::MODIFIERS`] and [`chord::KEYS`] resolves here, and what
//! each one *becomes* is this file's business alone. `cmd` landing on
//! `leftmeta` is a Linux convention — on macOS Command is its own modifier and
//! the chords an agent should send change with it — so the mapping lives
//! beside the keymap it feeds, and only the published names are shared.
//!
//! The reverse — that nothing *unpublished* resolves — is `chord::normalize`'s,
//! not this file's. `keysym::NAMED` is an X11 table and carries names like
//! `leftctrl` that this desktop understands and no other does; the gate refuses
//! them before they reach it, so the table stays a keymap rather than becoming
//! a second, accidental grammar.

use super::keysym;
use crate::chord;

/// Friendly key names → the internal names [`keysym::keysym_for`] knows.
///
/// `option` and `command` are here because a Mac keyboard's labels reach this
/// desktop too: an agent that learned `option+tab` on one host must not have
/// its chord refused on the other. They land on the same modifiers `alt` and
/// `super` do, which is what those keys do on a Linux desktop.
const ALIASES: &[(&str, &str)] = &[
    ("ctrl", "leftctrl"),
    ("control", "leftctrl"),
    ("alt", "leftalt"),
    ("option", "leftalt"),
    ("shift", "leftshift"),
    ("super", "leftmeta"),
    ("meta", "leftmeta"),
    ("win", "leftmeta"),
    ("cmd", "leftmeta"),
    ("command", "leftmeta"),
];

/// Resolve a chord to `(modifier mask, non-modifier keysyms)`.
///
/// Modifiers are aggregated into a bitmask announced through
/// `virtual_keyboard.modifiers`; the rest are pressed and released while that
/// mask is held.
pub(super) fn resolve(keys: &str) -> anyhow::Result<(u32, Vec<u32>)> {
    let mut mask = 0u32;
    let mut plain = Vec::new();

    for token in chord::normalize(keys, ALIASES)? {
        let sym = keysym::keysym_for(&token)?;
        match keysym::modifier_bit(sym) {
            Some(bit) => mask |= bit,
            None => plain.push(sym),
        }
    }

    Ok((mask, plain))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_friendly_modifier_names_to_their_left_variants() {
        assert_eq!(
            chord::normalize("Ctrl+CMD+ Return ", ALIASES).unwrap(),
            vec!["leftctrl", "leftmeta", "enter"],
        );
    }

    #[test]
    fn separates_the_modifier_mask_from_the_keys_it_holds() {
        let (mask, keys) = resolve("ctrl+shift+t").unwrap();
        assert_eq!(mask, 0x04 | 0x01, "Control | Shift");
        assert_eq!(keys, vec![0x74], "just `t`");
    }

    #[test]
    fn a_bare_key_carries_no_mask() {
        let (mask, keys) = resolve("enter").unwrap();
        assert_eq!(mask, 0);
        assert_eq!(keys, vec![0xFF0D]);
    }

    #[test]
    fn an_unknown_token_is_an_error_rather_than_a_silent_no_op() {
        assert!(resolve("ctrl+nonsense").is_err());
    }

    #[test]
    fn this_keymaps_own_spelling_is_not_a_second_grammar() {
        // `leftctrl` resolves through `keysym::NAMED` and would work here and
        // on no other desktop. The shared gate is what refuses it.
        assert!(resolve("leftctrl+a").is_err());
        assert!(resolve("rightmeta+a").is_err());
    }

    #[test]
    fn every_published_token_resolves() {
        // The published grammar is one contract over two backends. Before this
        // ran, `option` resolved on macOS and failed here, under a single tool
        // description that promised it on both.
        chord::every_published_token_resolves(resolve);
    }

    #[test]
    fn a_macs_labels_reach_the_modifiers_this_desktop_means() {
        let (alt, _) = resolve("option+tab").unwrap();
        assert_eq!(alt, 0x08, "option is this desktop's Alt");
        let (meta, _) = resolve("command+c").unwrap();
        assert_eq!(meta, 0x40, "command is this desktop's Super");
    }
}
