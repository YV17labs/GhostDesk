# Copyright (c) 2026 YV17 — MIT License
"""Accessibility wait tool — poll AT-SPI until an element appears."""

import asyncio

from mcp.server.fastmcp import FastMCP

from ghostdesk.tools.accessibility._client import run_atspi


async def wait_for_element(
    text: str,
    role: str | None = None,
    timeout_seconds: float = 10.0,
    poll_interval_ms: int = 500,
) -> dict:
    """Wait until a UI element appears on screen."""
    args = [text]
    if role:
        args.extend(["--role", role])

    loop = asyncio.get_event_loop()
    deadline = loop.time() + timeout_seconds

    while loop.time() < deadline:
        try:
            result = await run_atspi("find", args)
            if "center_x" in result:
                return {
                    "status": "ready",
                    "message": "Element found. Page is ready — proceed with your next action.",
                    "element": result,
                }
        except RuntimeError:
            pass
        await asyncio.sleep(poll_interval_ms / 1000.0)

    return {
        "status": "timeout",
        "error": f"Element '{text}' not found after {timeout_seconds}s",
    }


def register(mcp: FastMCP) -> None:
    """Register the wait_for_element tool."""
    mcp.tool()(wait_for_element)
