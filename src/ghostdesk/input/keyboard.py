# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Keyboard control tools — driven by the Wayland virtual-keyboard protocol.

All keyboard actions go through the singleton :class:`WaylandInput`,
which keeps a persistent ``zwp_virtual_keyboard_v1`` open and pushes an
on-the-fly XKB keymap so text entry is layout-independent: a French
AZERTY and a US QWERTY system layout produce the exact same output
because Ghostdesk never consults the system keymap.
"""

from __future__ import annotations

from mcp.server.fastmcp import Context

from ghostdesk._cursor import get_cursor_position
from ghostdesk.input._wayland import get_wayland_input
from ghostdesk.input.feedback import (
    build_feedback,
    capture_before,
    poll_for_change,
    warn_on_miss,
)

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


async def key_type(text: str, ctx: Context | None = None) -> dict:
    """Type text at the current keyboard focus. Handles Unicode, newlines,
    and tabs. Layout-independent — a French AZERTY host produces the same
    output as US QWERTY.

    For more than a sentence or two, prefer ``clipboard_set(text)`` +
    ``key_press("ctrl+v")``: it's instant, immune to autocomplete and
    autocorrect, and does not race with the app's own key handlers.

    A ``screen_changed: false`` result almost always means the field did
    not have focus. Click into it first and retry.

    Returns the standard ``{action, screen_changed, reaction_time_ms}``
    feedback.
    """
    cx, cy = get_cursor_position()
    region, before = await capture_before(cx, cy)

    wl = await get_wayland_input()
    await wl.type_text(text)

    result = await poll_for_change(region, before)
    feedback = build_feedback(f"Typed {len(text)} characters", result)
    await warn_on_miss(ctx, feedback)
    return feedback


async def key_press(keys: str, ctx: Context | None = None) -> dict:
    """Press a key or a chord (modifiers + key), using ``+`` as separator.

    Accepted modifier tokens: ``ctrl``/``control``, ``alt``, ``shift``,
    ``super``/``meta``/``win``/``cmd``.
    Accepted non-printable tokens: ``return``/``enter``, ``escape``/``esc``,
    ``backspace``, ``delete``, ``tab``, ``space``, ``home``/``end``,
    ``pageup``/``pagedown``, ``left``/``right``/``up``/``down``, ``f1``..``f12``.

    A ``screen_changed: false`` result usually means the keystroke went to
    a window or field that didn't care about it — check focus with a
    screenshot.

    Returns the standard ``{action, screen_changed, reaction_time_ms}``
    feedback.
    """
    cx, cy = get_cursor_position()
    region, before = await capture_before(cx, cy)

    tokens = _normalize_chord(keys)
    wl = await get_wayland_input()
    await wl.press_chord(tokens)

    result = await poll_for_change(region, before)
    feedback = build_feedback(f"Pressed {keys}", result)
    await warn_on_miss(ctx, feedback)
    return feedback
