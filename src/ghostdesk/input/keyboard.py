# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Keyboard control tools — driven by the Wayland virtual-keyboard protocol.

All keyboard actions go through the singleton :class:`WaylandInput`,
which keeps a persistent ``zwp_virtual_keyboard_v1`` open and pushes an
on-the-fly XKB keymap so text entry is layout-independent: a French
AZERTY and a US QWERTY system layout produce the exact same output
because Ghostdesk never consults the system keymap.
"""

from __future__ import annotations

from ghostdesk._cursor import get_cursor_position
from ghostdesk.input._wayland import get_wayland_input
from ghostdesk.input.feedback import build_feedback, capture_before, poll_for_change

# Map X11-style friendly names to our internal normalised key names
# (all lowercase, no underscores, ``leftctrl`` style for modifiers).
_KEY_ALIASES = {
    "ctrl": "leftctrl", "control": "leftctrl",
    "alt": "leftalt",
    "shift": "leftshift",
    "super": "leftmeta", "meta": "leftmeta", "win": "leftmeta", "cmd": "leftmeta",
    "return": "enter",
    "escape": "esc",
    "backspace": "backspace",
    "delete": "delete",
    "tab": "tab",
    "space": "space",
    "insert": "insert",
    "home": "home",
    "end": "end",
    "page_up": "pageup", "pageup": "pageup",
    "page_down": "pagedown", "pagedown": "pagedown",
    "left": "left", "right": "right", "up": "up", "down": "down",
}


def _normalize_token(token: str) -> str:
    """Convert a friendly key name to our internal key name."""
    key = token.strip().lower()
    if key in _KEY_ALIASES:
        return _KEY_ALIASES[key]
    if len(key) >= 2 and key[0] == "f" and key[1:].isdigit():
        return key
    return key


def _normalize_chord(keys: str) -> list[str]:
    """Convert ``Ctrl+Shift+Tab`` → ``["leftctrl", "leftshift", "tab"]``."""
    return [_normalize_token(t) for t in keys.split("+") if t.strip()]


async def type_text(text: str) -> dict:
    """Type text. Handles Unicode, newlines, and tabs.

    Returns the standard ``{action, screen_changed, reaction_time_ms}``
    feedback.  If ``screen_changed`` is false, the text field probably
    didn't have focus — click on it first and retry.
    """
    cx, cy = get_cursor_position()
    region, before = await capture_before(cx, cy)

    wl = await get_wayland_input()
    await wl.type_text(text)

    result = await poll_for_change(region, before)
    return build_feedback(f"Typed {len(text)} characters", result)


async def press_key(keys: str) -> dict:
    """Press a key or key combination.

    Friendly names accepted: ``Tab``, ``Return``, ``Escape``,
    ``BackSpace``, ``Left``, ``Page_Up``, ``F4``, ``Ctrl``, ``Alt``,
    ``Shift``, ``Super``.  Single printable characters stay as-is
    (``a``, ``c``, ``5``).

    Examples: ``Tab``, ``Ctrl+c``, ``Alt+F4``, ``Ctrl+Shift+Tab``.

    Returns the standard ``{action, screen_changed, reaction_time_ms}``
    feedback.
    """
    cx, cy = get_cursor_position()
    region, before = await capture_before(cx, cy)

    tokens = _normalize_chord(keys)
    wl = await get_wayland_input()
    await wl.press_chord(tokens)

    result = await poll_for_change(region, before)
    return build_feedback(f"Pressed {keys}", result)
