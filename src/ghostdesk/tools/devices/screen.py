# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screen tools — capture and display information."""

import asyncio
import os
import tempfile
from mcp.server.fastmcp import FastMCP, Image

from ghostdesk.utils.cmd import run
from ghostdesk.utils.clickables import get_clickables
from ghostdesk.utils.cursor_overlay import ImageFormat, draw_cursor
from ghostdesk.utils.humanizer import get_cursor_position


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

    Returns an image and a metadata object:

    - ``cursor`` — current pointer position ``{x, y}``.
    - ``apps`` — list of open applications with their clickable
      UI elements (buttons, links, fields…). May be empty when no
      application is running.
    """
    region = all(v is not None for v in (x, y, width, height))

    fd, path = tempfile.mkstemp(suffix=".png")
    os.close(fd)
    try:
        # Start AT-SPI query early — it's independent of the capture
        clickables_task = asyncio.create_task(get_clickables())

        cmd = ["maim", "--format=png"]
        if region:
            cmd += ["-g", f"{width}x{height}+{x}+{y}"]
        cmd.append(path)
        await run(cmd)

        with open(path, "rb") as f:
            raw_png = f.read()

        # Wait for cursor position and AT-SPI clickables
        (cx, cy), clickables = await asyncio.gather(
            get_cursor_position(), clickables_task,
        )

        if region:
            cx -= x  # type: ignore[operator]
            cy -= y  # type: ignore[operator]

        image_bytes = draw_cursor(
            raw_png, cx, cy,
            output_format=output_format,
            quality=quality,
        )

        metadata: dict = {"cursor": {"x": cx, "y": cy}, "apps": clickables}

        return [
            Image(data=image_bytes, format=output_format),
            metadata,
        ]
    finally:
        try:
            os.unlink(path)
        except FileNotFoundError:
            pass


def register(mcp: FastMCP) -> None:
    """Register screenshot-related tools on the MCP server."""
    mcp.tool()(screenshot)
