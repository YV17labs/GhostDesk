# Copyright (c) 2026 YV17 — MIT License
"""Accessibility read tools — read the screen like a screen reader via AT-SPI."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.accessibility._client import VALID_ROLES, run_atspi


async def read_screen(
    role: str | None = None,
    max_results: int = 500,
) -> dict:
    """Read all visible UI elements (name, role, states) in reading order."""
    args = ["--max", str(max_results)]
    if role:
        if role not in VALID_ROLES:
            raise ValueError(
                f"Invalid role '{role}'. Must be one of: {', '.join(sorted(VALID_ROLES))}"
            )
        args.extend(["--role", role])
    return await run_atspi("read", args)


async def get_element_details(
    text: str,
    role: str | None = None,
) -> dict:
    """Inspect one UI element in detail (states, actions, value, children)."""
    args = [text]
    if role:
        args.extend(["--role", role])
    return await run_atspi("details", args)


async def read_table(
    text: str | None = None,
    max_rows: int = 100,
) -> dict:
    """Extract a structured table (headers + rows as arrays)."""
    args = ["--max-rows", str(max_rows)]
    if text:
        args.extend(["--text", text])
    return await run_atspi("table", args)


def register(mcp: FastMCP) -> None:
    """Register accessibility read tools."""
    mcp.tool()(read_screen)
    mcp.tool()(get_element_details)
    mcp.tool()(read_table)
