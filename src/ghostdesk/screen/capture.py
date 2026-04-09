# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screenshot capture tool."""

import asyncio
import hashlib
import io
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
from ghostdesk.screen.annotations import draw_detections
from ghostdesk.screen.detector import detect
from ghostdesk.screen.windows import get_open_windows

ImageFormat = Literal["webp", "png"]


def _image_hash(image_bytes: bytes) -> str:
    """Compute hash of image bytes for comparison."""
    return hashlib.md5(image_bytes).hexdigest()


def _enlarge_region(region: Region, scale: int = 3) -> Region:
    """Enlarge region by scale factor, centered on original, clamped to screen bounds.

    Args:
        region: Original region
        scale: Enlargement factor (default 3x)

    Returns:
        Enlarged region clamped to screen bounds
    """
    center_x = region.x + region.width // 2
    center_y = region.y + region.height // 2

    new_width = region.width * scale
    new_height = region.height * scale

    new_x = max(0, center_x - new_width // 2)
    new_y = max(0, center_y - new_height // 2)

    # Clamp to screen bounds
    if new_x + new_width > SCREEN_WIDTH:
        new_x = max(0, SCREEN_WIDTH - new_width)
    if new_y + new_height > SCREEN_HEIGHT:
        new_y = max(0, SCREEN_HEIGHT - new_height)

    return Region(x=new_x, y=new_y, width=new_width, height=new_height)


async def screenshot(
    region: Region | None = None,
    format: ImageFormat = "png",
    detect: bool = False,
    stabilize: bool = True,
) -> list:
    """Capture the screen with optional UI element detection.

    By default returns the raw screenshot. With detect=True and a region,
    automatically enlarges region 3x and uses GPA-GUI-Detector to annotate
    UI elements.

    Args:
        region: Optional area to capture (full screen if omitted).
        format: "png" or "webp".
        detect: Auto-detect UI elements using GPA-GUI-Detector.
            Only active when region is provided. Automatically enlarges region 3x.
        stabilize: Wait for page to stabilize before capturing (max 5 sec).
            Compares successive screenshots to detect page movement.

    Returns: [Image, JSON metadata (screen, cursor, windows)].
    """
    # Determine capture region and offset
    capture_region = region
    offset_x = 0
    offset_y = 0

    if detect and region:
        # Enlarge region 3x and capture the enlarged area
        enlarged_region = _enlarge_region(region, scale=3)
        capture_region = enlarged_region
        offset_x = enlarged_region.x
        offset_y = enlarged_region.y
    elif region:
        offset_x = region.x
        offset_y = region.y

    # Clip region to screen bounds
    if capture_region:
        clipped_region = Region(
            x=max(0, min(capture_region.x, SCREEN_WIDTH)),
            y=max(0, min(capture_region.y, SCREEN_HEIGHT)),
            width=max(0, min(capture_region.width, SCREEN_WIDTH - capture_region.x)),
            height=max(0, min(capture_region.height, SCREEN_HEIGHT - capture_region.y)),
        )
        capture_region = clipped_region

    # Stabilize page if requested
    if stabilize:
        raw_png = await _capture_until_stable(capture_region)
    else:
        raw_png = await capture_png(capture_region)

    cursor_task = asyncio.create_task(get_cursor_position())
    windows_task = asyncio.create_task(get_open_windows())

    (cx, cy), windows = await asyncio.gather(cursor_task, windows_task)

    # Apply detection if requested
    if detect and region:
        pil_img = PILImage.open(io.BytesIO(raw_png)).convert("RGB")
        detections = await detect(pil_img, offset_x=offset_x, offset_y=offset_y)
        img_bytes = draw_detections(raw_png, detections, fmt=format)
    else:
        img_bytes = _reencode(raw_png, format)

    metadata = build_metadata(cx, cy, windows, region)

    return [Image(data=img_bytes, format=format), metadata]


async def _capture_until_stable(region: Region | None = None) -> bytes:
    """Capture screenshot repeatedly until page stabilizes.

    Compares successive screenshots (as webp) to detect if page is still moving.
    Max wait time is 5 seconds.

    Args:
        region: Optional area to capture.

    Returns:
        PNG bytes of the stable screenshot.
    """
    import time
    MAX_WAIT = 5.0
    POLL_INTERVAL = 0.3

    start_time = time.time()
    prev_hash = None

    while True:
        raw_png = await capture_png(region)
        # Convert to webp for comparison (more efficient)
        webp_bytes = _reencode(raw_png, "webp")
        current_hash = _image_hash(webp_bytes)

        # If hash matches previous capture, page is stable
        if prev_hash is not None and current_hash == prev_hash:
            return raw_png

        # Check timeout
        elapsed = time.time() - start_time
        if elapsed >= MAX_WAIT:
            return raw_png

        # Wait before next capture
        prev_hash = current_hash
        await asyncio.sleep(POLL_INTERVAL)


def _reencode(raw_png: bytes, fmt: ImageFormat) -> bytes:
    """Re-encode raw PNG bytes into the requested format."""
    if fmt == "png":
        return raw_png
    import io
    from PIL import Image as PILImage
    img = PILImage.open(io.BytesIO(raw_png))
    return save_image_bytes(img, fmt)
