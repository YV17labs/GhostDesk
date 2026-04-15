# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Clipboard domain — read and write system clipboard."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.clipboard.clipboard_get import clipboard_get
from ghostdesk.clipboard.clipboard_set import clipboard_set


def register(mcp: FastMCP) -> None:
    """Register clipboard tools."""
    mcp.tool()(clipboard_get)
    mcp.tool()(clipboard_set)
