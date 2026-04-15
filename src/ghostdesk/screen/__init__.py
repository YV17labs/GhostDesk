# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Screen domain — capture and analyze the display."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.screen.screen_shot import screen_shot


def register(mcp: FastMCP) -> None:
    """Register screen tools."""
    mcp.tool()(screen_shot)
