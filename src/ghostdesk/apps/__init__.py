# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Apps domain — application launching and process tracking."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.apps.app_launch import app_launch
from ghostdesk.apps.app_list import app_list
from ghostdesk.apps.app_status import app_status


def register(mcp: FastMCP) -> None:
    """Register apps tools."""
    mcp.tool()(app_list)
    mcp.tool()(app_launch)
    mcp.tool()(app_status)
