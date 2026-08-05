//! X11 keysym resolution.
//!
//! Only the symbolic names GhostDesk actually needs, derived from
//! `X11/keysymdef.h`. Names mirror what the keyboard tool normalises to:
//! lowercase, no underscores, `leftctrl` style for modifiers.

/// Keysym → XKB modifier bit.
///
/// Modifier state is announced to the compositor through
/// `zwp_virtual_keyboard_v1::modifiers(mask, …)` rather than by pressing
/// modifier keysyms as ordinary keys — that is what the protocol expects, and
/// it sidesteps keymap-interpretation quirks entirely.
pub const MODIFIER_BITS: &[(u32, u32)] = &[
    (0xFFE1, 0x01), // Shift_L   → Shift
    (0xFFE2, 0x01), // Shift_R   → Shift
    (0xFFE3, 0x04), // Control_L → Control
    (0xFFE4, 0x04), // Control_R → Control
    (0xFFE9, 0x08), // Alt_L     → Mod1
    (0xFFEA, 0x08), // Alt_R     → Mod1
    (0xFFEB, 0x40), // Super_L   → Mod4
    (0xFFEC, 0x40), // Super_R   → Mod4
];

const NAMED: &[(&str, u32)] = &[
    ("leftctrl", 0xFFE3),
    ("rightctrl", 0xFFE4),
    ("leftshift", 0xFFE1),
    ("rightshift", 0xFFE2),
    ("leftalt", 0xFFE9),
    ("rightalt", 0xFFEA),
    ("leftmeta", 0xFFEB),
    ("rightmeta", 0xFFEC),
    ("enter", 0xFF0D),
    ("esc", 0xFF1B),
    ("backspace", 0xFF08),
    ("tab", 0xFF09),
    ("delete", 0xFFFF),
    ("insert", 0xFF63),
    ("home", 0xFF50),
    ("end", 0xFF57),
    ("pageup", 0xFF55),
    ("pagedown", 0xFF56),
    ("left", 0xFF51),
    ("up", 0xFF52),
    ("right", 0xFF53),
    ("down", 0xFF54),
    ("space", 0x0020),
];

/// The X11 modifier bit for `keysym`, when it is a modifier at all.
pub fn modifier_bit(keysym: u32) -> Option<u32> {
    MODIFIER_BITS
        .iter()
        .find_map(|(sym, bit)| (*sym == keysym).then_some(*bit))
}

fn named(token: &str) -> Option<u32> {
    let lower = token.to_lowercase();
    if let Some((_, sym)) = NAMED.iter().find(|(name, _)| *name == lower) {
        return Some(*sym);
    }
    // f1 … f12 — XK_F1 is 0xFFBE, so the base is 0xFFBD.
    let n: u32 = lower.strip_prefix('f')?.parse().ok()?;
    (1..=12).contains(&n).then(|| 0xFFBD + n)
}

/// Resolve a friendly key name or a single character to its X11 keysym.
///
/// ```text
/// keysym_for("leftctrl") → 0xFFE3
/// keysym_for("a")        → 0x61
/// keysym_for("é")        → 0xE9
/// keysym_for("€")        → 0x010020AC   (Unicode keysym range)
/// ```
pub fn keysym_for(token: &str) -> anyhow::Result<u32> {
    if let Some(sym) = named(token) {
        return Ok(sym);
    }

    let mut chars = token.chars();
    let (Some(ch), None) = (chars.next(), chars.next()) else {
        anyhow::bail!("unknown key name: {token:?}");
    };
    Ok(keysym_for_char(ch))
}

/// The keysym for one character.
///
/// Newline and tab have no clean Unicode keysym — they need their X11
/// functional ones, or the compositor types nothing at all.
pub fn keysym_for_char(ch: char) -> u32 {
    match ch {
        '\n' => 0xFF0D, // Return
        '\t' => 0xFF09, // Tab
        _ => {
            let cp = ch as u32;
            if cp < 0x100 {
                cp
            } else {
                // X11 convention for Unicode beyond latin1.
                cp | 0x0100_0000
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_named_keys_case_insensitively() {
        assert_eq!(keysym_for("leftctrl").unwrap(), 0xFFE3);
        assert_eq!(keysym_for("Enter").unwrap(), 0xFF0D);
        assert_eq!(keysym_for("f1").unwrap(), 0xFFBE);
        assert_eq!(keysym_for("f12").unwrap(), 0xFFC9);
    }

    #[test]
    fn rejects_an_f_key_outside_the_table() {
        assert!(keysym_for("f13").is_err());
        assert!(keysym_for("fnord").is_err());
    }

    #[test]
    fn maps_characters_through_the_unicode_convention() {
        assert_eq!(keysym_for("a").unwrap(), 0x61);
        assert_eq!(keysym_for("é").unwrap(), 0xE9);
        assert_eq!(keysym_for("€").unwrap(), 0x0100_20AC);
    }

    #[test]
    fn whitespace_uses_functional_keysyms() {
        assert_eq!(keysym_for_char('\n'), 0xFF0D);
        assert_eq!(keysym_for_char('\t'), 0xFF09);
    }

    #[test]
    fn only_modifiers_carry_a_bit() {
        assert_eq!(modifier_bit(0xFFE3), Some(0x04));
        assert_eq!(modifier_bit(0xFFEB), Some(0x40));
        assert_eq!(modifier_bit(0x61), None);
    }
}
