# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Input domain — mouse and keyboard control."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.input.keyboard import key_press, key_type
from ghostdesk.input.mouse import (
    mouse_click,
    mouse_double_click,
    mouse_drag,
    mouse_scroll,
)


def register(mcp: FastMCP) -> None:
    """Register input tools."""
    mcp.tool()(mouse_click)
    mcp.tool()(mouse_double_click)
    mcp.tool()(mouse_drag)
    mcp.tool()(mouse_scroll)
    mcp.tool()(key_type)
    mcp.tool()(key_press)
