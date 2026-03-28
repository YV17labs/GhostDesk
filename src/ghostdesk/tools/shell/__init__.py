# Copyright (c) 2026 YV17 — MIT License
"""Shell channel — command execution, app launching, and timing."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.shell import exec_, launch


def register(mcp: FastMCP) -> None:
    """Register all shell-channel tools."""
    exec_.register(mcp)
    launch.register(mcp)
