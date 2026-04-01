# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Shell channel — app launching and timing."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.shell import launch


def register(mcp: FastMCP) -> None:
    """Register all shell-channel tools."""
    launch.register(mcp)
