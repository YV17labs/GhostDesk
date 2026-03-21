# Copyright (c) 2026 YV17 — MIT License
"""Accessibility interact tools — manipulate UI elements via AT-SPI (pure).

All tools in this module use only the AT-SPI accessibility API.  They do not
depend on xdotool or the humanizer.  The click_element tool, which bridges
to xdotool, lives in click.py.
"""

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.accessibility._client import run_atspi


async def set_value(
    text: str,
    value: str,
    role: str | None = None,
) -> dict:
    """Set the text content or numeric value of a UI element.

    For text fields: clears existing content and sets the new value.
    For sliders/spinbuttons: sets the numeric value.

    This is more reliable than click + type_text because it uses the
    accessibility API directly — no need to focus, select all, then type.

    Args:
        text: Text to search for in element names to find the target.
        value: The value to set (string for text fields, number as string for sliders).
        role: Optional role filter to narrow the search.
    """
    args = [text, value]
    if role:
        args.extend(["--role", role])
    return await run_atspi("set-value", args)


async def focus_element(
    text: str,
    role: str | None = None,
) -> dict:
    """Give keyboard focus to a UI element.

    After focusing, you can use type_text() or press_key() to interact
    with the element. Useful for elements that need focus before they
    accept keyboard input.

    Args:
        text: Text to search for in element names (case-insensitive substring).
        role: Optional role filter to narrow the search.
    """
    args = [text]
    if role:
        args.extend(["--role", role])
    return await run_atspi("focus", args)


async def scroll_to_element(
    text: str,
    role: str | None = None,
) -> dict:
    """Scroll to bring a UI element into view.

    Use this when you know an element exists but it's not visible on screen
    (e.g., it's below the fold). This is more reliable than mouse_scroll()
    because it targets a specific element.

    Args:
        text: Text to search for in element names (case-insensitive substring).
        role: Optional role filter to narrow the search.
    """
    args = [text]
    if role:
        args.extend(["--role", role])
    return await run_atspi("scroll", args)


def register(mcp: FastMCP) -> None:
    """Register AT-SPI–pure interaction tools."""
    mcp.tool()(set_value)
    mcp.tool()(focus_element)
    mcp.tool()(scroll_to_element)
