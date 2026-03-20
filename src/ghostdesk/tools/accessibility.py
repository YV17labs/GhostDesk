# Copyright (c) 2026 YV17 — MIT License
"""Accessibility tools — AT-SPI powered element discovery and interaction."""

import asyncio
import json
import os

from mcp.server.fastmcp import FastMCP

from ghostdesk.utils.humanizer import human_move, get_cursor_position
from ghostdesk.utils.xdotool import run

# Path to the AT-SPI query script, executed with system Python (has python3-gi).
_ATSPI_SCRIPT = os.path.join(
    os.path.dirname(os.path.dirname(__file__)), "utils", "atspi_query.py",
)

# Valid role names (must match _ROLE_NAMES values in atspi_query.py).
_VALID_ROLES = frozenset({
    "button", "toggle", "checkbox", "radio", "combobox", "menu", "menuitem",
    "link", "textfield", "password", "text", "spinbutton", "slider", "tab",
    "treeitem", "listitem", "cell",
})


def _parse_role(role: str | None) -> set[str] | None:
    """Validate and convert a role string to a filter set."""
    if not role:
        return None
    if role not in _VALID_ROLES:
        raise ValueError(
            f"Invalid role '{role}'. Must be one of: {', '.join(sorted(_VALID_ROLES))}"
        )
    return {role}


async def _query_atspi(
    role_filter: set[str] | None = None,
    max_results: int = 200,
) -> list[dict]:
    """Call the AT-SPI query helper and return parsed elements."""
    cmd = ["/usr/bin/python3", _ATSPI_SCRIPT, "--max", str(max_results)]
    for r in role_filter or ():
        cmd.extend(["--role", r])

    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        env={**os.environ, "DISPLAY": os.environ.get("DISPLAY", ":99")},
    )
    stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=10.0)

    if proc.returncode != 0:
        msg = stderr.decode(errors="replace").strip()
        raise RuntimeError(f"AT-SPI query failed: {msg}")

    data = json.loads(stdout.decode())
    if isinstance(data, dict) and "error" in data:
        raise RuntimeError(data["error"])

    return data


def register(mcp: FastMCP) -> None:
    """Register accessibility-related tools on the MCP server."""

    @mcp.tool()
    async def list_elements(
        role: str | None = None,
        max_results: int = 100,
    ) -> list[dict]:
        """List interactive UI elements visible on screen using the accessibility tree.

        Returns elements with their role, name, position and size — no screenshot
        or OCR needed. Use this to discover what is clickable before acting.

        Each element includes center_x / center_y coordinates that can be passed
        directly to mouse_click().

        Args:
            role: Optional filter — one of: button, toggle, checkbox, radio,
                  combobox, menu, menuitem, link, textfield, password, text,
                  spinbutton, slider, tab, treeitem, listitem, cell.
                  If omitted, all interactive elements are returned.
            max_results: Maximum number of elements (default 100).
        """
        return await _query_atspi(role_filter=_parse_role(role), max_results=max_results)

    @mcp.tool()
    async def click_element(
        text: str,
        role: str | None = None,
        humanize: bool = True,
    ) -> str:
        """Find a UI element by its name and click on its center.

        Searches the accessibility tree for an element whose name contains the
        given text (case-insensitive). Clicks the first match.

        Args:
            text: Text to search for in element names (case-insensitive substring match).
            role: Optional role filter to narrow the search (e.g. "button", "link").
            humanize: If True (default), move with a realistic curve before clicking.
        """
        elements = await _query_atspi(role_filter=_parse_role(role))

        text_lower = text.lower()
        match = None
        for el in elements:
            name = el.get("name", "")
            desc = el.get("description", "")
            if text_lower in name.lower() or text_lower in desc.lower():
                match = el
                break

        if match is None:
            available = [e["name"] for e in elements[:20] if e.get("name")]
            return (
                f"No element found matching '{text}'."
                f" Available elements: {available}"
            )

        x, y = match["center_x"], match["center_y"]

        if humanize:
            cx, cy = await get_cursor_position()
            await human_move(cx, cy, x, y)
        else:
            await run(["xdotool", "mousemove", str(x), str(y)])

        await run(["xdotool", "click", "1"])

        return (
            f"Clicked '{match['name']}' ({match['role']}) "
            f"at ({x}, {y})"
        )
