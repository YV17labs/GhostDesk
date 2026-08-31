//! Key names → Windows virtual-key codes.
//!
//! Two halves, and they are answered differently. The named keys are a fixed
//! table covering exactly [`chord::KEYS`](crate::chord::KEYS) and nothing
//! more — a key only this desktop answered to would be a chord the other two
//! hosts refuse, which the published grammar has no way to express.
//!
//! A printable key is *not* a table. `VkKeyScanW` asks the active keyboard
//! layout which key carries a character, so `ctrl+a` presses the key that
//! types `a` — the physical `Q` on an AZERTY board — rather than whichever
//! key sits where `A` is on a US one. That is the same property the Wayland
//! backend gets from generating its own XKB keymap, and the reason neither
//! host needs the caller to know the layout.
//!
//! Free text does not come through here at all: `type_text` injects Unicode
//! directly, so nothing in this file has to describe a character being typed.

#![expect(
    unsafe_code,
    reason = "VkKeyScanW is C; the call carries its own SAFETY note"
)]

use anyhow::{Result, bail};

use ::windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5,
    VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12, VK_HOME, VK_LEFT, VK_NEXT, VK_PRIOR,
    VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP, VkKeyScanW,
};

/// Named non-printable keys.
///
/// `delete` is `VK_DELETE`, the key above the arrows, and `backspace` is
/// `VK_BACK`. Windows names the second one after the character it once sent;
/// GhostDesk publishes both under the names the rest of the world uses.
const NAMED: &[(&str, VIRTUAL_KEY)] = &[
    ("enter", VK_RETURN),
    ("tab", VK_TAB),
    ("space", VK_SPACE),
    ("backspace", VK_BACK),
    ("delete", VK_DELETE),
    ("esc", VK_ESCAPE),
    ("f1", VK_F1),
    ("f2", VK_F2),
    ("f3", VK_F3),
    ("f4", VK_F4),
    ("f5", VK_F5),
    ("f6", VK_F6),
    ("f7", VK_F7),
    ("f8", VK_F8),
    ("f9", VK_F9),
    ("f10", VK_F10),
    ("f11", VK_F11),
    ("f12", VK_F12),
    ("home", VK_HOME),
    ("end", VK_END),
    ("pageup", VK_PRIOR),
    ("pagedown", VK_NEXT),
    ("left", VK_LEFT),
    ("right", VK_RIGHT),
    ("up", VK_UP),
    ("down", VK_DOWN),
];

/// What `VkKeyScanW` packs into the high byte of its answer.
const NEEDS_SHIFT: i16 = 0x01;
const NEEDS_CTRL: i16 = 0x02;
const NEEDS_ALT: i16 = 0x04;

/// The virtual key for one normalised chord token, and whether reaching it
/// needs Shift held.
///
/// The Shift half is not optional bookkeeping. On an AZERTY layout the digits
/// live above the letters *under* Shift, so `ctrl+1` is three keys and not
/// two — and a backend that dropped the Shift would silently send `ctrl+&`.
pub(super) fn for_token(token: &str) -> Result<(VIRTUAL_KEY, bool)> {
    if let Some((_, code)) = NAMED.iter().find(|(name, _)| *name == token) {
        return Ok((*code, false));
    }

    let mut chars = token.chars();
    let (Some(ch), None) = (chars.next(), chars.next()) else {
        bail!("unknown key name: {token:?}")
    };

    let mut utf16 = [0u16; 2];
    let [unit] = ch.encode_utf16(&mut utf16) else {
        // Outside the basic plane there is no key to press — such a character
        // is typed, never chorded.
        bail!("no key on this layout carries {ch:?}")
    };

    // SAFETY: takes a UTF-16 unit by value and returns a packed integer.
    let scanned = unsafe { VkKeyScanW(*unit) };
    if scanned == -1 {
        bail!("no key on this keyboard layout carries {ch:?}");
    }

    let state = scanned >> 8;
    if state & (NEEDS_CTRL | NEEDS_ALT) != 0 {
        // AltGr characters (`@` on AZERTY, `{` on QWERTZ) reach their key only
        // through modifiers a chord already spends on its own meaning. Refused
        // rather than approximated: `ctrl+@` would otherwise press three
        // modifiers and mean none of them.
        bail!("{ch:?} needs AltGr on this keyboard layout, so it cannot be part of a chord");
    }

    Ok((
        VIRTUAL_KEY((scanned & 0xff) as u16),
        state & NEEDS_SHIFT != 0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_named_keys_without_a_layout() {
        assert_eq!(for_token("enter").unwrap(), (VK_RETURN, false));
        assert_eq!(for_token("f5").unwrap(), (VK_F5, false));
    }

    #[test]
    fn the_two_delete_keys_do_not_collide() {
        assert_eq!(for_token("backspace").unwrap().0, VK_BACK);
        assert_eq!(for_token("delete").unwrap().0, VK_DELETE);
    }

    #[test]
    fn an_unknown_token_is_an_error_rather_than_a_silent_no_op() {
        assert!(for_token("nonsense").is_err());
        assert!(for_token("").is_err());
    }
}
