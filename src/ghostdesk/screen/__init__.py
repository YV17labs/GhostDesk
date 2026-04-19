# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Screen domain — capture and analyze the display."""

from mcp.server.fastmcp import FastMCP
from mcp.types import ToolAnnotations

from ghostdesk.screen.screen_shot import screen_shot


def register(mcp: FastMCP) -> None:
    """Register screen tools."""
    mcp.tool(
        annotations=ToolAnnotations(readOnlyHint=True, idempotentHint=True),
    )(screen_shot)
