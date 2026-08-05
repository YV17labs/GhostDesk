//! On-the-fly XKB keymap generation.
//!
//! Same trick `wtype` uses: build a minimal keymap where every keysym we need
//! gets its own dedicated keycode. Text entry then becomes
//! layout-independent — a US QWERTY host, a French AZERTY and a Dvorak all
//! produce identical output, because the compositor's system keymap is never
//! consulted.

use std::fmt::Write as _;

/// Slot `i` maps XKB keycode `8 + i` (evdev keycode `i`) to `keysyms[i]` as a
/// `ONE_LEVEL` key: no shift/lock/modifier level switching. Modifier state is
/// announced separately through `virtual_keyboard.modifiers`, so no
/// `modifier_map` is emitted.
pub fn build_keymap(keysyms: &[u32]) -> String {
    if keysyms.is_empty() {
        // Empty keymaps are legal — the compositor just gets a no-op layout.
        return concat!(
            "xkb_keymap {\n",
            "  xkb_keycodes \"ghostdesk\" { minimum = 8; maximum = 8; };\n",
            "  xkb_types \"ghostdesk\" {\n",
            "    virtual_modifiers Ghostdesk;\n",
            "    type \"ONE_LEVEL\" { modifiers = none; level_name[Level1] = \"Any\"; };\n",
            "  };\n",
            "  xkb_compatibility \"ghostdesk\" {};\n",
            "  xkb_symbols \"ghostdesk\" {};\n",
            "};\n",
        )
        .to_string();
    }

    let last = 8 + keysyms.len() - 1;

    let mut keycodes = String::new();
    let mut symbols = String::new();
    for (slot, keysym) in keysyms.iter().enumerate() {
        let _ = writeln!(keycodes, "    <K{slot}> = {};", 8 + slot);
        let _ = writeln!(
            symbols,
            "    key <K{slot}> {{ type = \"ONE_LEVEL\", symbols[Group1] = [ 0x{keysym:08x} ] }};"
        );
    }

    format!(
        "xkb_keymap {{\n\
         \x20 xkb_keycodes \"ghostdesk\" {{\n\
         \x20   minimum = 8;\n\
         \x20   maximum = {last};\n\
         {keycodes}\
         \x20 }};\n\
         \x20 xkb_types \"ghostdesk\" {{\n\
         \x20   virtual_modifiers Ghostdesk;\n\
         \x20   type \"ONE_LEVEL\" {{\n\
         \x20     modifiers = none;\n\
         \x20     level_name[Level1] = \"Any\";\n\
         \x20   }};\n\
         \x20 }};\n\
         \x20 xkb_compatibility \"ghostdesk\" {{}};\n\
         \x20 xkb_symbols \"ghostdesk\" {{\n\
         {symbols}\
         \x20 }};\n\
         }};\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_pool_still_produces_a_valid_layout() {
        let keymap = build_keymap(&[]);
        assert!(keymap.starts_with("xkb_keymap {"));
        assert!(keymap.contains("minimum = 8; maximum = 8;"));
        assert!(keymap.trim_end().ends_with("};"));
    }

    #[test]
    fn each_keysym_gets_its_own_keycode_starting_at_eight() {
        let keymap = build_keymap(&[0x61, 0xFF0D]);
        assert!(keymap.contains("<K0> = 8;"));
        assert!(keymap.contains("<K1> = 9;"));
        assert!(keymap.contains("maximum = 9;"));
        assert!(
            keymap.contains("key <K0> { type = \"ONE_LEVEL\", symbols[Group1] = [ 0x00000061 ] };")
        );
        assert!(
            keymap.contains("key <K1> { type = \"ONE_LEVEL\", symbols[Group1] = [ 0x0000ff0d ] };")
        );
    }

    #[test]
    fn no_modifier_map_is_emitted() {
        // Modifier state travels through `virtual_keyboard.modifiers`, so a
        // modifier_map here would be a second, conflicting source of truth.
        assert!(!build_keymap(&[0xFFE3]).contains("modifier_map"));
    }
}
