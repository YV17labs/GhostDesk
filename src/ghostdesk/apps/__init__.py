# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Apps domain — application launching and process tracking."""

from mcp.server.fastmcp import FastMCP
from mcp.types import ToolAnnotations

from ghostdesk.apps.app_launch import app_launch
from ghostdesk.apps.app_list import app_list
from ghostdesk.apps.app_status import app_status


def register(mcp: FastMCP) -> None:
    """Register apps tools."""
    mcp.tool(
        annotations=ToolAnnotations(readOnlyHint=True, idempotentHint=True),
    )(app_list)
    mcp.tool(
        annotations=ToolAnnotations(destructiveHint=False),
    )(app_launch)
    mcp.tool(
        annotations=ToolAnnotations(readOnlyHint=True),
    )(app_status)
