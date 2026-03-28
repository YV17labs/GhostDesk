# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Clipboard channel — read and write the system clipboard."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.clipboard import get, set_


def register(mcp: FastMCP) -> None:
    """Register all clipboard tools."""
    get.register(mcp)
    set_.register(mcp)
