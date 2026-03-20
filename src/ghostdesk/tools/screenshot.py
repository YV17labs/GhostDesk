# Copyright (c) 2026 YV17 — MIT License
"""Screen capture tools — with cursor overlay and window metadata."""

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
        fd, path = tempfile.mkstemp(suffix=".png")
        os.close(fd)
        try:
            cmd = ["maim", "--format=png"]
            if all(v is not None for v in (x, y, width, height)):
                cmd += ["-g", f"{width}x{height}+{x}+{y}"]
            cmd.append(path)
            await run(cmd)

            with open(path, "rb") as f:
                raw_png = f.read()

            # Draw cursor on the image
            cx, cy = await get_cursor_position()
            if all(v is not None for v in (x, y, width, height)):
                # Adjust cursor coords relative to the captured region
                cx -= x  # type: ignore[operator]
                cy -= y  # type: ignore[operator]
            png_with_cursor = draw_cursor(raw_png, cx, cy)

            # Gather window metadata (best-effort)
            win_info = await get_window_info()

            return [
                Image(data=png_with_cursor, format="png"),
                {
                    "cursor": {"x": cx, "y": cy},
                    "active_window": win_info["active_window"],
                    "windows": win_info["windows"],
                },
            ]
        finally:
            if os.path.exists(path):
                os.unlink(path)

    @mcp.tool()
    async def get_screen_size() -> dict[str, int]:
        """Return the current screen width and height in pixels."""
        output = await run(["xdotool", "getdisplaygeometry"])
        parts = output.split()
        return {"width": int(parts[0]), "height": int(parts[1])}
