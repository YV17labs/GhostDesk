# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screenshot capture tool."""

import asyncio
import io
import time
from typing import Literal

from PIL import Image as PILImage
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
from ghostdesk.screen.windows import get_open_windows

ImageFormat = Literal["webp", "png"]


async def screenshot(
    region: Region | None = None,
    format: ImageFormat = "webp",
    stabilize: bool = True,
) -> list:
    """Capture the screen, optionally cropped to a region.

    Args:
        region: Area to capture (full screen if omitted).
        format: "webp" (default, smaller payload) or "png" (lossless).
        stabilize: Wait for the page to stop moving before capturing
            (max 5 s). Useful right after navigation.

    Returns: ``[Image, metadata]`` where metadata holds screen size,
    cursor position, and the list of open windows.
    """
    capture_region = region
    if capture_region:
        capture_region = Region(
            x=max(0, min(capture_region.x, SCREEN_WIDTH)),
            y=max(0, min(capture_region.y, SCREEN_HEIGHT)),
            width=max(0, min(capture_region.width, SCREEN_WIDTH - capture_region.x)),
            height=max(0, min(capture_region.height, SCREEN_HEIGHT - capture_region.y)),
        )

    if stabilize:
        raw_png = await _capture_until_stable(capture_region)
    else:
        raw_png = await capture_png(capture_region)

    (cx, cy), windows = await asyncio.gather(
        get_cursor_position(), get_open_windows(),
    )

    img_bytes = _reencode(raw_png, format)
    metadata = build_metadata(cx, cy, windows, region)

    return [Image(data=img_bytes, format=format), metadata]


async def _capture_until_stable(region: Region | None = None) -> bytes:
    """Capture screenshot repeatedly until page stabilizes.

    Compares successive raw PNG bytes from maim — they're deterministic for
    identical pixels, so a bytewise equality check is the cheapest way to
    confirm two consecutive captures are identical. Max wait time is 5 s.
    """
    MAX_WAIT = 5.0
    POLL_INTERVAL = 0.3

    start_time = time.time()
    prev = None

    while True:
        raw_png = await capture_png(region)
        if prev is not None and raw_png == prev:
            return raw_png
        if time.time() - start_time >= MAX_WAIT:
            return raw_png
        prev = raw_png
        await asyncio.sleep(POLL_INTERVAL)


def _reencode(raw_png: bytes, fmt: ImageFormat) -> bytes:
    """Re-encode raw PNG bytes into the requested format."""
    if fmt == "png":
        return raw_png
    img = PILImage.open(io.BytesIO(raw_png))
    return save_image_bytes(img, fmt)
