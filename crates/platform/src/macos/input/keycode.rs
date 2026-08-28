//! Key names → macOS virtual keycodes.
//!
//! The numbers are Carbon's `kVK_*` constants (`HIToolbox/Events.h`), which
//! are positional: keycode 0 is wherever `A` sits on a US ANSI board, whatever
//! the user's layout actually produces there. That is fine for the chords
//! GhostDesk publishes — they are named by *function* (`cmd+c`, `f5`, `tab`),
//! and the OS resolves a positional code through the active layout.
//!
//! Free text does not come through here at all: `type_text` injects Unicode
//! directly, so nothing in this table has to describe a character.
//!
//! The named half covers exactly [`chord::KEYS`](crate::chord::KEYS) and
//! nothing more: a key only this desktop answers to is a chord the Linux host
//! refuses, which the published grammar has no way to express.

use anyhow::{Result, bail};
use objc2_core_graphics::CGKeyCode;

/// Named non-printable keys, in `kVK_*` order.
const NAMED: &[(&str, CGKeyCode)] = &[
    ("enter", 36),
    ("tab", 48),
    ("space", 49),
    // kVK_Delete is the key labelled ⌫; kVK_ForwardDelete is the ⌦ above the
    // arrows. GhostDesk publishes them under the names the rest of the world
    // uses, not the ones Apple's header does.
    ("backspace", 51),
    ("delete", 117),
    ("esc", 53),
    ("f1", 122),
    ("f2", 120),
    ("f3", 99),
    ("f4", 118),
    ("f5", 96),
    ("f6", 97),
    ("f7", 98),
    ("f8", 100),
    ("f9", 101),
    ("f10", 109),
    ("f11", 103),
    ("f12", 111),
    ("home", 115),
    ("end", 119),
    ("pageup", 116),
    ("pagedown", 121),
    ("left", 123),
    ("right", 124),
    ("down", 125),
    ("up", 126),
];

/// Printable keys on the US ANSI layout, by the character they carry
/// unshifted.
const PRINTABLE: &[(char, CGKeyCode)] = &[
    ('a', 0),
    ('s', 1),
    ('d', 2),
    ('f', 3),
    ('h', 4),
    ('g', 5),
    ('z', 6),
    ('x', 7),
    ('c', 8),
    ('v', 9),
    ('b', 11),
    ('q', 12),
    ('w', 13),
    ('e', 14),
    ('r', 15),
    ('y', 16),
    ('t', 17),
    ('1', 18),
    ('2', 19),
    ('3', 20),
    ('4', 21),
    ('6', 22),
    ('5', 23),
    ('=', 24),
    ('9', 25),
    ('7', 26),
    ('-', 27),
    ('8', 28),
    ('0', 29),
    (']', 30),
    ('o', 31),
    ('u', 32),
    ('[', 33),
    ('i', 34),
    ('p', 35),
    ('l', 37),
    ('j', 38),
    ('\'', 39),
    ('k', 40),
    (';', 41),
    ('\\', 42),
    (',', 43),
    ('/', 44),
    ('n', 45),
    ('m', 46),
    ('.', 47),
    ('`', 50),
];

/// The virtual keycode for one normalised chord token.
pub(super) fn for_token(token: &str) -> Result<CGKeyCode> {
    if let Some((_, code)) = NAMED.iter().find(|(name, _)| *name == token) {
        return Ok(*code);
    }

    let mut chars = token.chars();
    if let (Some(ch), None) = (chars.next(), chars.next())
        && let Some((_, code)) = PRINTABLE.iter().find(|(key, _)| *key == ch)
    {
        return Ok(*code);
    }

    bail!("unknown key name: {token:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_named_keys_and_single_characters() {
        assert_eq!(for_token("enter").unwrap(), 36);
        assert_eq!(for_token("f5").unwrap(), 96);
        assert_eq!(for_token("c").unwrap(), 8);
        assert_eq!(for_token("0").unwrap(), 29);
    }

    #[test]
    fn the_two_delete_keys_do_not_collide() {
        assert_eq!(for_token("backspace").unwrap(), 51, "the ⌫ key");
        assert_eq!(for_token("delete").unwrap(), 117, "the ⌦ key");
    }

    #[test]
    fn an_unknown_token_is_an_error_rather_than_a_silent_no_op() {
        assert!(for_token("nonsense").is_err());
        assert!(for_token("").is_err());
    }
}
