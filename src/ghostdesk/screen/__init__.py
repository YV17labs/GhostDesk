# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screen domain — capture and analyze the display."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.screen.capture import screenshot


def register(mcp: FastMCP) -> None:
    """Register screen tools."""
    mcp.tool()(screenshot)
