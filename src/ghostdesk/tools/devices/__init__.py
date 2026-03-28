# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Devices channel — mouse, keyboard, and screen capture via X11/xdotool."""

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.devices import keyboard, mouse, screen


def register(mcp: FastMCP) -> None:
    """Register all device tools."""
    mouse.register(mcp)
    keyboard.register(mcp)
    screen.register(mcp)
