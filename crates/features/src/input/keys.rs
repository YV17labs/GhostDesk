//! Chord parsing — `Ctrl+Shift+Tab` → the keysyms and modifier mask the
//! virtual keyboard needs.

use platform::wayland::keysym;

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

/// Normalise one token to its internal key name.
pub fn normalize_token(token: &str) -> String {
    let key = token.trim().to_lowercase();
    ALIASES
        .iter()
        .find_map(|(from, to)| (*from == key).then(|| (*to).to_string()))
        .unwrap_or(key)
}

/// Split a chord on `+`, normalising each token and dropping blanks.
pub fn normalize_chord(keys: &str) -> Vec<String> {
    keys.split('+')
        .filter(|token| !token.trim().is_empty())
        .map(normalize_token)
        .collect()
}

/// Resolve a chord to `(modifier mask, non-modifier keysyms)`.
///
/// Modifiers are aggregated into a bitmask announced through
/// `virtual_keyboard.modifiers`; the rest are pressed and released while that
/// mask is held.
pub fn resolve_chord(keys: &str) -> anyhow::Result<(u32, Vec<u32>)> {
    let mut mask = 0u32;
    let mut plain = Vec::new();

    for token in normalize_chord(keys) {
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
        assert_eq!(normalize_token("Ctrl"), "leftctrl");
        assert_eq!(normalize_token("CMD"), "leftmeta");
        assert_eq!(normalize_token(" Return "), "enter");
    }

    #[test]
    fn splits_a_chord_and_drops_blanks() {
        assert_eq!(
            normalize_chord("Ctrl+Shift+Tab"),
            vec!["leftctrl", "leftshift", "tab"],
        );
        assert_eq!(normalize_chord("ctrl++a"), vec!["leftctrl", "a"]);
    }

    #[test]
    fn separates_the_modifier_mask_from_the_keys_it_holds() {
        let (mask, keys) = resolve_chord("ctrl+shift+t").unwrap();
        assert_eq!(mask, 0x04 | 0x01, "Control | Shift");
        assert_eq!(keys, vec![0x74], "just `t`");
    }

    #[test]
    fn a_bare_key_carries_no_mask() {
        let (mask, keys) = resolve_chord("enter").unwrap();
        assert_eq!(mask, 0);
        assert_eq!(keys, vec![0xFF0D]);
    }

    #[test]
    fn an_unknown_token_is_an_error_rather_than_a_silent_no_op() {
        assert!(resolve_chord("ctrl+nonsense").is_err());
    }
}
