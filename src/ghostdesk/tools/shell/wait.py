# Copyright (c) 2026 YV17 — MIT License
"""Shell wait tool — timed pause between actions."""

import asyncio

from mcp.server.fastmcp import FastMCP


async def wait(milliseconds: int) -> str:
    """Wait for a specified number of milliseconds.

    Useful to let animations finish, pages load, or UI transitions complete
    before taking a screenshot or performing the next action.

    Args:
        milliseconds: Duration to wait.
    """
    await asyncio.sleep(milliseconds / 1000.0)
    return f"Waited {milliseconds}ms"


def register(mcp: FastMCP) -> None:
    """Register the wait tool."""
    mcp.tool()(wait)
