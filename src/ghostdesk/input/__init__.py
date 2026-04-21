# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Input domain — mouse and keyboard control."""

from mcp.server.fastmcp import FastMCP
from mcp.types import ToolAnnotations

from ghostdesk._icons import GHOSTDESK_ICONS
from ghostdesk.input.keyboard import key_press, key_type
from ghostdesk.input.mouse import (
    mouse_click,
    mouse_double_click,
    mouse_drag,
    mouse_move,
    mouse_scroll,
)

_DESTRUCTIVE = ToolAnnotations(destructiveHint=True)
_VIEWPORT = ToolAnnotations(destructiveHint=False)


def register(mcp: FastMCP) -> None:
    """Register input tools."""
    mcp.tool(annotations=_VIEWPORT, icons=GHOSTDESK_ICONS)(mouse_move)
    mcp.tool(annotations=_DESTRUCTIVE, icons=GHOSTDESK_ICONS)(mouse_click)
    mcp.tool(annotations=_DESTRUCTIVE, icons=GHOSTDESK_ICONS)(mouse_double_click)
    mcp.tool(annotations=_DESTRUCTIVE, icons=GHOSTDESK_ICONS)(mouse_drag)
    mcp.tool(annotations=_VIEWPORT, icons=GHOSTDESK_ICONS)(mouse_scroll)
    mcp.tool(annotations=_DESTRUCTIVE, icons=GHOSTDESK_ICONS)(key_type)
    mcp.tool(annotations=_DESTRUCTIVE, icons=GHOSTDESK_ICONS)(key_press)
