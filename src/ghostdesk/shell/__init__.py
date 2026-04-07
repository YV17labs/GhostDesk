# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Shell domain — application launching and process tracking."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.shell.launch import launch
from ghostdesk.shell.process_status import process_status


def register(mcp: FastMCP) -> None:
    """Register shell tools."""
    mcp.tool()(launch)
    mcp.tool()(process_status)
