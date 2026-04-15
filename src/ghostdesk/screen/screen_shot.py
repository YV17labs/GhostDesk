# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Screen screen_shot tool — capture the desktop display."""

import asyncio
import io
import time
from typing import Literal

from PIL import Image as PILImage
from mcp.server.fastmcp import Image

from ghostdesk.screen._shared import (
    Region,
    SCREEN_HEIGHT,
    SCREEN_WIDTH,
    capture_png,
    save_image_bytes,
    screens_stable,
)

ImageFormat = Literal["webp", "png"]

_STABILITY_TIMEOUT_S = 2.5
_STABILITY_POLL_S = 0.15


async def screen_shot(
    region: Region | None = None,
    format: ImageFormat = "webp",
    stabilize: bool = True,
) -> Image:
    """Capture the screen, optionally cropped to a region.

    Args:
        region: Area to capture (full screen if omitted).
        format: "webp" (default, smaller payload) or "png" (lossless).
        stabilize: Wait for the page to stop moving before capturing
            (max 2.5 s). Useful right after navigation.
    """
    capture_region = _clamp_region(region)

    if stabilize:
        raw_png = await _capture_until_stable(capture_region)
    else:
        raw_png = await capture_png(capture_region)

    img_bytes = _reencode(raw_png, format)
    return Image(data=img_bytes, format=format)


def _clamp_region(region: Region | None) -> Region | None:
    """Clamp a Region to the screen bounds so grim never sees negatives."""
    if region is None:
        return None
    x = max(0, min(region.x, SCREEN_WIDTH))
    y = max(0, min(region.y, SCREEN_HEIGHT))
    w = max(0, min(region.width, SCREEN_WIDTH - x))
    h = max(0, min(region.height, SCREEN_HEIGHT - y))
    return Region(x, y, w, h)


async def _capture_until_stable(region: Region | None = None) -> bytes:
    """Poll grim until two consecutive frames are pixel-stable.

    Gives up after :data:`_STABILITY_TIMEOUT_S` and returns the latest
    frame regardless — a genuinely animating screen shouldn't block the
    agent forever.
    """
    prev = await capture_png(region)
    deadline = time.monotonic() + _STABILITY_TIMEOUT_S
    while time.monotonic() < deadline:
        curr = await capture_png(region)
        if screens_stable(prev, curr):
            return curr
        await asyncio.sleep(_STABILITY_POLL_S)
        prev = curr
    return prev


def _reencode(raw_png: bytes, fmt: ImageFormat) -> bytes:
    """Re-encode raw PNG bytes into the requested format."""
    if fmt == "png":
        return raw_png
    img = PILImage.open(io.BytesIO(raw_png))
    return save_image_bytes(img, fmt)
