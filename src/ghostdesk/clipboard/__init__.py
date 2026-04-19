# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Clipboard domain — read and write system clipboard."""

from mcp.server.fastmcp import FastMCP
from mcp.types import ToolAnnotations

from ghostdesk._icons import GHOSTDESK_ICONS
from ghostdesk.clipboard.clipboard_get import clipboard_get
from ghostdesk.clipboard.clipboard_set import clipboard_set


def register(mcp: FastMCP) -> None:
    """Register clipboard tools."""
    mcp.tool(
        annotations=ToolAnnotations(readOnlyHint=True, idempotentHint=True),
        icons=GHOSTDESK_ICONS,
    )(clipboard_get)
    mcp.tool(
        annotations=ToolAnnotations(destructiveHint=False, idempotentHint=True),
        icons=GHOSTDESK_ICONS,
    )(clipboard_set)
