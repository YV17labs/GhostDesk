# Copyright (c) 2026 YV17 — MIT License
"""Accessibility read tools — read screen content via AT-SPI."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.accessibility._client import VALID_ROLES, run_atspi


async def list_elements(
    role: str | None = None,
    max_results: int = 100,
) -> list[dict]:
    """List interactive UI elements visible on screen (buttons, links, fields…).

    Returns elements with their role, name, position and size — no screenshot
    or OCR needed. Use this to discover what you can click, type into, or
    interact with.

    Each element includes center_x / center_y coordinates that can be passed
    directly to mouse_click().

    Args:
        role: Optional filter — one of: button, toggle, checkbox, radio,
              combobox, menu, menuitem, link, textfield, password, text,
              spinbutton, slider, tab, treeitem, listitem, cell.
              If omitted, all interactive elements are returned.
        max_results: Maximum number of elements (default 100).
    """
    args = ["--max", str(max_results)]
    if role:
        if role not in VALID_ROLES:
            raise ValueError(
                f"Invalid role '{role}'. Must be one of: {', '.join(sorted(VALID_ROLES))}"
            )
        args.extend(["--role", role])
    return await run_atspi("elements", args)


async def read_screen(
    max_results: int = 500,
) -> list[dict]:
    """Read all visible text content from the screen.

    Returns headings, paragraphs, labels, link text, list items, and other
    text content — everything a human would read on screen. Results are
    sorted top-to-bottom in natural reading order.

    This is faster and more accurate than taking a screenshot to read text.
    Use this instead of screenshot() when you need to know what text is
    displayed.

    Each entry includes: role (heading, paragraph, label, link, etc.),
    text content, and y/x position for ordering.

    Args:
        max_results: Maximum number of text elements (default 500).
    """
    return await run_atspi("text", ["--max", str(max_results)])


async def get_element_details(
    text: str,
    role: str | None = None,
) -> dict:
    """Inspect a UI element in detail: its text, states, available actions,
    value, relations, attributes, and children.

    Use this when you need more information about a specific element than
    list_elements provides — for example, to check if a checkbox is checked,
    what actions are available on a button, the current value of a slider,
    or to read the full text content of an element.

    Args:
        text: Text to search for in element names (case-insensitive substring).
        role: Optional role filter to narrow the search.
    """
    args = [text]
    if role:
        args.extend(["--role", role])
    return await run_atspi("details", args)


async def read_table(
    text: str | None = None,
    max_rows: int = 100,
) -> dict:
    """Extract a structured table from the screen.

    Returns headers and rows as arrays, making it easy to read spreadsheet
    data, HTML tables, or any tabular content without screenshots.

    The result includes: name, row/column counts, headers array, and a
    2D data array of cell values.

    Args:
        text: Optional table name or caption to search for. If omitted,
              returns the first table found on screen.
        max_rows: Maximum rows to extract (default 100).
    """
    args = ["--max-rows", str(max_rows)]
    if text:
        args.extend(["--text", text])
    return await run_atspi("table", args)


def register(mcp: FastMCP) -> None:
    """Register accessibility read tools."""
    mcp.tool()(list_elements)
    mcp.tool()(read_screen)
    mcp.tool()(get_element_details)
    mcp.tool()(read_table)
