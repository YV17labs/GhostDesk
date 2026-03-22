# Copyright (c) 2026 YV17 — MIT License
"""Accessibility interact tools — manipulate UI elements via AT-SPI (pure).

All tools in this module use only the AT-SPI accessibility API.  They do not
depend on xdotool or the humanizer.  The click_element tool, which bridges
to xdotool, lives in click.py.
"""

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.accessibility._client import run_atspi

_NOT_FOUND_HINT = "Element not found. Use read_screen() to see available elements."


async def _run_or_hint(cmd: str, args: list[str]) -> dict:
    """Run an AT-SPI command; add a hint if the result contains an error."""
    try:
        result = await run_atspi(cmd, args)
    except RuntimeError as e:
        return {"error": str(e), "hint": _NOT_FOUND_HINT}
    if isinstance(result, dict) and "error" in result:
        result["hint"] = _NOT_FOUND_HINT
    return result


async def set_value(
    text: str,
    value: str,
    role: str | None = None,
) -> dict:
    """Set text or numeric value on a form field."""
    args = [text, value]
    if role:
        args.extend(["--role", role])
    return await _run_or_hint("set-value", args)


async def focus_element(
    text: str,
    role: str | None = None,
) -> dict:
    """Give keyboard focus to a UI element."""
    args = [text]
    if role:
        args.extend(["--role", role])
    return await _run_or_hint("focus", args)


async def scroll_to_element(
    text: str,
    role: str | None = None,
) -> dict:
    """Scroll an off-screen element into view."""
    args = [text]
    if role:
        args.extend(["--role", role])
    return await _run_or_hint("scroll", args)


def register(mcp: FastMCP) -> None:
    """Register AT-SPI–pure interaction tools."""
    mcp.tool()(set_value)
    mcp.tool()(focus_element)
    mcp.tool()(scroll_to_element)
