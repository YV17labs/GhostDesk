# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screen tools — capture and display information."""

import asyncio
import os
import tempfile
from mcp.server.fastmcp import FastMCP, Image

from ghostdesk.utils.cmd import run
from ghostdesk.utils.cursor_overlay import ImageFormat, draw_cursor
from ghostdesk.utils.humanizer import get_cursor_position
from ghostdesk.utils.window_info import get_window_info


async def screenshot(
    x: int | None = None,
    y: int | None = None,
    width: int | None = None,
    height: int | None = None,
    output_format: ImageFormat = "png",
    quality: int = 80,
) -> list:
    """Capture the screen as an image.

    *output_format* — ``"png"`` (default, lossless) or ``"webp"``
    (lossy, ~2-3× smaller).  *quality* is only used for WebP (1-100, default 80).
    """
    region = all(v is not None for v in (x, y, width, height))

    fd, path = tempfile.mkstemp(suffix=".png")
    os.close(fd)
    try:
        cmd = ["maim", "--format=png"]
        if region:
            cmd += ["-g", f"{width}x{height}+{x}+{y}"]
        cmd.append(path)
        await run(cmd)

        with open(path, "rb") as f:
            raw_png = f.read()

        # Cursor position and window metadata in parallel
        (cx, cy), win_info = await asyncio.gather(
            get_cursor_position(), get_window_info(),
        )

        if region:
            cx -= x  # type: ignore[operator]
            cy -= y  # type: ignore[operator]

        image_bytes = draw_cursor(
            raw_png, cx, cy,
            output_format=output_format,
            quality=quality,
        )

        return [
            Image(data=image_bytes, format=output_format),
            {
                "cursor": {"x": cx, "y": cy},
                "active_window": win_info["active_window"],
                "windows": win_info["windows"],
            },
        ]
    finally:
        try:
            os.unlink(path)
        except FileNotFoundError:
            pass


def register(mcp: FastMCP) -> None:
    """Register screenshot-related tools on the MCP server."""
    mcp.tool()(screenshot)
