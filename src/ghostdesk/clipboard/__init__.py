# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Clipboard domain — read and write system clipboard."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.clipboard.get import get_clipboard
from ghostdesk.clipboard.set_ import set_clipboard


def register(mcp: FastMCP) -> None:
    """Register clipboard tools."""
    mcp.tool()(get_clipboard)
    mcp.tool()(set_clipboard)
