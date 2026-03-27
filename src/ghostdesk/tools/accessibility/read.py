# Copyright (c) 2026 YV17 — MIT License
"""Accessibility read tools — read the screen like a screen reader via AT-SPI."""

import asyncio

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.accessibility._client import VALID_ROLES, run_atspi
from ghostdesk.utils.window_info import get_active_window

# Common aliases → canonical AT-SPI role names.
_ROLE_ALIASES: dict[str, str] = {
    "row": "table_row",
    "column": "table_column_header",
    "input": "textfield",
    "dropdown": "combobox",
    "header": "heading",
}


def _resolve_role(role: str) -> str:
    """Resolve a role alias to its canonical AT-SPI name, or validate as-is."""
    canonical = _ROLE_ALIASES.get(role, role)
    if canonical not in VALID_ROLES:
        raise ValueError(
            f"Invalid role '{role}'. Must be one of: {', '.join(sorted(VALID_ROLES))}"
        )
    return canonical


async def read_screen(
    role: str | None = None,
    app: str | None = None,
    max_results: int = 100,
    include_positions: bool = True,
) -> dict:
    """Read visible UI elements as a flat list sorted by position. Browser chrome is separated automatically. Use *role* to filter (e.g. role='row' for lists, 'input' for forms). Use *app* to restrict to one application."""
    args = ["--max", str(max_results)]
    if include_positions:
        args.append("--include-positions")
    if role:
        args.extend(["--role", _resolve_role(role)])
    if app:
        args.extend(["--app", app])

    result, active = await asyncio.gather(
        run_atspi("read", args),
        get_active_window(),
    )
    result["active_window"] = active

    return result


def register(mcp: FastMCP) -> None:
    """Register accessibility read tools."""
    mcp.tool()(read_screen)
