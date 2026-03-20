# Copyright (c) 2026 YV17 — MIT License
"""Keyboard control tools — with optional human-like typing."""

import asyncio

from mcp.server.fastmcp import FastMCP

from ghostdesk.utils.humanizer import typing_delays
from ghostdesk.utils.xdotool import run


_CHAR_TO_KEY = {"\n": "Return", "\t": "Tab"}


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


def register(mcp: FastMCP) -> None:
    """Register keyboard-related tools on the MCP server."""

    @mcp.tool()
    async def type_text(text: str, delay_ms: int = 50, humanize: bool = True) -> str:
        """Type text character by character, as if using a physical keyboard.

        Args:
            text: The string to type.
            delay_ms: Base milliseconds between keystrokes. Defaults to 50.
                      Ignored when humanize=False (uses 12ms fixed).
            humanize: If True (default), vary timing per character like a real
                      typist — faster mid-word, slower after spaces/punctuation.
        """
        if humanize:
            delays = typing_delays(text, base_delay_ms=delay_ms)
            for char, delay in zip(text, delays):
                await _type_char(char)
                await asyncio.sleep(delay)
        else:
            for char in text:
                await _type_char(char)

        return f"Typed {len(text)} characters"

    @mcp.tool()
    async def press_key(keys: str) -> str:
        """Press a key or key combination.

        Examples: 'Return', 'ctrl+c', 'alt+F4', 'shift+ctrl+t', 'Tab'.
        Use xdotool key names (XKeysym).

        Args:
            keys: Key or combo string like 'ctrl+s' or 'Return'.
        """
        await run(["xdotool", "key", "--clearmodifiers", keys])
        return f"Pressed {keys}"
