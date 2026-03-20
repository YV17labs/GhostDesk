# Copyright (c) 2026 YV17 — MIT License
"""Screen capture tools — with cursor overlay and window metadata."""

import asyncio
import os
import tempfile

from mcp.server.fastmcp import FastMCP, Image

from ghostdesk.utils.cursor_overlay import draw_cursor
from ghostdesk.utils.humanizer import get_cursor_position
from ghostdesk.utils.window_info import get_window_info
from ghostdesk.utils.xdotool import run


def register(mcp: FastMCP) -> None:
    """Register screenshot-related tools on the MCP server."""

    @mcp.tool()
    async def screenshot(
        x: int | None = None,
        y: int | None = None,
        width: int | None = None,
        height: int | None = None,
    ) -> list:
        """Capture the screen and return the image with cursor visible.

        The screenshot always includes a bright crosshair showing the
        current cursor position, plus metadata about open windows.

        Call with no arguments for a full-screen capture, or pass x, y,
        width, and height to capture a specific region.
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

            png_with_cursor = draw_cursor(raw_png, cx, cy)

            return [
                Image(data=png_with_cursor, format="png"),
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

    @mcp.tool()
    async def get_screen_size() -> dict[str, int]:
        """Return the current screen width and height in pixels."""
        output = await run(["xdotool", "getdisplaygeometry"])
        parts = output.split()
        return {"width": int(parts[0]), "height": int(parts[1])}
