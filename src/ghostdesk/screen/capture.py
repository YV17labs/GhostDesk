# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screenshot capture tool."""

import asyncio
from typing import Literal

from mcp.server.fastmcp import Image

from ghostdesk._cursor import get_cursor_position
from ghostdesk.screen._shared import (
    Region,
    SCREEN_HEIGHT,
    SCREEN_WIDTH,
    build_metadata,
    capture_png,
    save_image_bytes,
)
from ghostdesk.screen.rulers import draw_rulers
from ghostdesk.screen.windows import get_open_windows

ImageFormat = Literal["webp", "png"]


async def screenshot(
    region: Region | None = None,
    format: ImageFormat = "png",
    rulers: bool = False,
) -> list:
    """Capture the screen with optional coordinate rulers.

    By default returns the raw screenshot for clarity. Coordinates are always
    absolute screen coordinates, even with `region=`.

    Args:
        region: Optional area to capture (full screen if omitted).
        format: "png" or "webp".
        rulers: Draw coordinate rulers on edges (X-axis top, Y-axis left)
            with marks every 20 pixels. Recommended for precise clicking.

    Returns: [Image, JSON metadata (screen, cursor, windows)].
    """
    cursor_task = asyncio.create_task(get_cursor_position())
    windows_task = asyncio.create_task(get_open_windows())

    # Clip region to screen bounds to avoid capturing black edges
    if region:
        clipped_region = Region(
            x=max(0, min(region.x, SCREEN_WIDTH)),
            y=max(0, min(region.y, SCREEN_HEIGHT)),
            width=max(0, min(region.width, SCREEN_WIDTH - region.x)),
            height=max(0, min(region.height, SCREEN_HEIGHT - region.y)),
        )
        region = clipped_region

    raw_png = await capture_png(region)

    (cx, cy), windows = await asyncio.gather(cursor_task, windows_task)

    if rulers and region:
        offset_x = region.x
        offset_y = region.y
        img_bytes = draw_rulers(
            raw_png, offset_x=offset_x, offset_y=offset_y, fmt=format,
        )
    else:
        img_bytes = _reencode(raw_png, format)

    metadata = build_metadata(cx, cy, windows, region)

    return [Image(data=img_bytes, format=format), metadata]


def _reencode(raw_png: bytes, fmt: ImageFormat) -> bytes:
    """Re-encode raw PNG bytes into the requested format."""
    if fmt == "png":
        return raw_png
    import io
    from PIL import Image as PILImage
    img = PILImage.open(io.BytesIO(raw_png))
    return save_image_bytes(img, fmt)
