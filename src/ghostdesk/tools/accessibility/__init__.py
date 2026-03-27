# Copyright (c) 2026 YV17 — MIT License
"""Accessibility channel — AT-SPI powered reading and interaction."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.accessibility import read, wait_for


def register(mcp: FastMCP) -> None:
    """Register all accessibility-channel tools."""
    read.register(mcp)
    wait_for.register(mcp)
