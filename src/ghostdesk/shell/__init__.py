# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Shell domain — application launching."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.shell.launch import launch


def register(mcp: FastMCP) -> None:
    """Register shell tools."""
    mcp.tool()(launch)
