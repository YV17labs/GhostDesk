# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Keyboard control tools — with optional human-like typing and visual feedback."""

import asyncio
import random

from ghostdesk._cmd import run
from ghostdesk._cursor import get_cursor_position
from ghostdesk.input.feedback import build_feedback, capture_before, poll_for_change

_CHAR_TO_KEY = {"\n": "Return", "\t": "Tab"}


def _normalize_keys(keys: str) -> str:
    """Normalize a key combo so X11 keysyms match xdotool expectations.

    Multi-character tokens get their first letter uppercased
    (``tab`` → ``Tab``, ``escape`` → ``Escape``, ``ctrl`` →
    ``Ctrl``). xdotool's multi-letter keysyms are case-sensitive
    but modifier names aren't, so uppercasing works for both.

    Single-character tokens are passed through untouched — the
    caller is in control: ``a`` stays ``a``, ``A`` stays ``A``
    (which xdotool reads as Shift+a). Tokens already containing
    a mix of cases (``BackSpace``, ``XF86AudioPlay``) are also
    left alone.
    """
    parts = keys.split("+")
    normalized = []
    for part in parts:
        if len(part) <= 1 or part != part.lower():
            normalized.append(part)
        else:
            normalized.append(part[0].upper() + part[1:])
    return "+".join(normalized)


def _typing_delays(text: str, base_delay_ms: int = 50) -> list[float]:
    """Generate human-like per-character delays (in seconds)."""
    delays: list[float] = []
    for char in text:
        if char == " ":
            ms = random.gauss(base_delay_ms * 1.5, base_delay_ms * 0.3)
        elif char in ".,;:!?\n":
            ms = random.gauss(base_delay_ms * 2.5, base_delay_ms * 0.5)
        else:
            ms = random.gauss(base_delay_ms, base_delay_ms * 0.2)
        delays.append(max(10, ms) / 1000.0)
    return delays


async def _type_char(char: str) -> None:
    """Type a single character, using the appropriate xdotool method."""
    key = _CHAR_TO_KEY.get(char)
    if key:
        await run(["xdotool", "key", "--clearmodifiers", key])
    elif char.isascii():
        await run(["xdotool", "type", "--clearmodifiers", "--delay", "0", char])
    else:
        # xdotool type can't handle non-ASCII on US keyboard layout.
        # Use xdotool key with Unicode codepoint (e.g., U00E8 for è).
        codepoint = f"U{ord(char):04X}"
        await run(["xdotool", "key", "--clearmodifiers", codepoint])


async def type_text(text: str, delay_ms: int = 50, humanize: bool = True) -> dict:
    """Type text character by character with human-like timing.

    Returns the standard ``{action, screen_changed, reaction_time_ms}``
    feedback. If ``screen_changed`` is false, the text field probably
    didn't have focus — click on it first and retry.
    """
    cx, cy = await get_cursor_position()
    region, before = await capture_before(cx, cy)

    if humanize:
        delays = _typing_delays(text, base_delay_ms=delay_ms)
        for char, delay in zip(text, delays):
            await _type_char(char)
            await asyncio.sleep(delay)
    else:
        for char in text:
            await _type_char(char)

    result = await poll_for_change(region, before)
    return build_feedback(f"Typed {len(text)} characters", result)


async def press_key(keys: str) -> dict:
    """Press a key or key combination.

    Multi-character tokens must start with an uppercase letter:
    ``Tab``, ``Return``, ``Escape``, ``BackSpace``, ``Left``,
    ``Page_Up``, ``F4``, ``Ctrl``, ``Alt``, ``Shift``, ``Super``.
    Single-character keys stay lowercase (``a``, ``c``, ``5``) —
    uppercasing them would imply Shift.

    Examples: ``Tab``, ``Ctrl+c``, ``Alt+F4``, ``Ctrl+Shift+Tab``.

    Returns the standard ``{action, screen_changed, reaction_time_ms}``
    feedback.
    """
    cx, cy = await get_cursor_position()
    region, before = await capture_before(cx, cy)

    normalized = _normalize_keys(keys)
    await run(["xdotool", "key", "--clearmodifiers", normalized])

    result = await poll_for_change(region, before)
    return build_feedback(f"Pressed {normalized}", result)
