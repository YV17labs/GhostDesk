# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Keyboard control tools — with optional human-like typing and visual feedback."""

import asyncio
import random

from ghostdesk._cmd import run
from ghostdesk._cursor import get_cursor_position
from ghostdesk.input.feedback import build_feedback, capture_before, poll_for_change

_CHAR_TO_KEY = {"\n": "Return", "\t": "Tab"}


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

    Captures a 200×200 px zone around the current mouse cursor before and
    after typing.

    Returns a dict with:
    - action: description of what was performed.
    - screen_changed: whether the zone visibly changed within 2 s. If false
      the text field may not have focus — click on it first.
    - reaction_time_ms: how quickly the change was detected (ms).
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
    """Press a key or key combination (e.g. 'Return', 'ctrl+c', 'alt+F4').

    Captures a 200×200 px zone around the current mouse cursor before and
    after the key press.

    Returns a dict with:
    - action: description of what was performed.
    - screen_changed: whether the zone visibly changed within 2 s.
    - reaction_time_ms: how quickly the change was detected (ms).
    """
    cx, cy = await get_cursor_position()
    region, before = await capture_before(cx, cy)

    await run(["xdotool", "key", "--clearmodifiers", keys])

    result = await poll_for_change(region, before)
    return build_feedback(f"Pressed {keys}", result)
