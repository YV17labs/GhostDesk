//! Chord parsing — `Ctrl+Shift+Tab` → the keysyms and modifier mask the
//! virtual keyboard needs.
//!
//! This vocabulary is deliberately backend-private: `cmd` mapping to
//! `leftmeta` is a *Linux* convention (on macOS, Cmd is its own modifier and
//! the chords an agent should send change with it), so the table lives beside
//! the keymap it feeds, not in the feature layer.

use super::keysym;
use crate::input::normalize_chord;

/// Friendly key names → the internal names [`keysym::keysym_for`] knows.
const ALIASES: &[(&str, &str)] = &[
    ("ctrl", "leftctrl"),
    ("control", "leftctrl"),
    ("alt", "leftalt"),
    ("shift", "leftshift"),
    ("super", "leftmeta"),
    ("meta", "leftmeta"),
    ("win", "leftmeta"),
    ("cmd", "leftmeta"),
    ("return", "enter"),
    ("escape", "esc"),
    ("page_up", "pageup"),
    ("page_down", "pagedown"),
];

/// Resolve a chord to `(modifier mask, non-modifier keysyms)`.
///
/// Modifiers are aggregated into a bitmask announced through
/// `virtual_keyboard.modifiers`; the rest are pressed and released while that
/// mask is held.
pub(super) fn resolve(keys: &str) -> anyhow::Result<(u32, Vec<u32>)> {
    let mut mask = 0u32;
    let mut plain = Vec::new();

    for token in normalize_chord(keys, ALIASES) {
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
            normalize_chord("Ctrl+CMD+ Return ", ALIASES),
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
}
