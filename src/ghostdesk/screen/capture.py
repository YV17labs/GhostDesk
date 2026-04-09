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
from ghostdesk.screen.detector import detect as detect_elements
from ghostdesk.screen.windows import get_open_windows

ImageFormat = Literal["webp", "png"]


def _image_hash(image_bytes: bytes) -> str:
    """Compute hash of image bytes for comparison."""
    return hashlib.md5(image_bytes).hexdigest()


_DETECT_TARGET_WIDTH = 360
_DETECT_TARGET_HEIGHT = 280


def _adaptive_confidence(capture_region: Region | None) -> float:
    """Pick a detector confidence threshold based on the captured area size.

    Tight zooms can afford a strict threshold (the user already targeted a
    precise area, so we want few/clean detections). Big captures need a
    loose threshold so the model isn't filtered into silence on a busy
    full screen.

    Returns a value between 0.10 and 0.40.
    """
    if capture_region is None:
        return 0.10
    area = capture_region.width * capture_region.height
    screen_area = SCREEN_WIDTH * SCREEN_HEIGHT
    ratio = area / screen_area if screen_area else 1.0
    if ratio < 0.10:
        return 0.40   # tight zoom — precision mode
    if ratio < 0.50:
        return 0.20   # mid-size capture — balanced
    return 0.10       # large / full screen — recall mode


def _enlarge_region(
    region: Region,
    target_width: int = _DETECT_TARGET_WIDTH,
    target_height: int = _DETECT_TARGET_HEIGHT,
) -> Region:
    """Adaptively pad a region up to a minimum capture size.

    The smaller the requested region, the more padding we add — both to
    give the detector enough pixels and to absorb LLM aiming errors. A
    region that is already at or above the target size is returned
    unchanged: large regions don't need a safety margin since the LLM
    already gave us plenty of context.

    The padded region is centered on the original and clamped to screen
    bounds.
    """
    new_width = max(region.width, target_width)
    new_height = max(region.height, target_height)

    if new_width == region.width and new_height == region.height:
        return region

    center_x = region.x + region.width // 2
    center_y = region.y + region.height // 2

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
    confidence: float | None = None,
) -> list:
    """Capture the screen, optionally with UI element detection.

    Without ``detect``, returns the raw screen (or region) as is.

    With ``detect=True`` and a ``region``, the region is adaptively padded
    up to a minimum capture size (small/precise regions get a large safety
    margin, regions already big enough are kept untouched), GPA-GUI-Detector
    runs on the captured area, and every detected element is drawn on the
    image as a colored box with a label showing its center point in absolute
    screen coordinates (``x,y``). Those labels are the literal click points
    to feed to ``mouse_click``.

    Args:
        region: Area to capture (full screen if omitted).
        format: "png" or "webp".
        detect: Run UI element detection on the captured area. Works
            on a ``region`` (with adaptive padding) or on the full screen.
        stabilize: Wait for the page to stop moving before capturing
            (max 5 s). Useful right after navigation.
        confidence: Detector confidence threshold (0..1). Lower = more
            sensitive, more false positives. If omitted, an adaptive
            value is chosen based on capture size: ~0.40 for tight zooms,
            ~0.20 for mid-size captures, ~0.10 for full-screen.

    Returns: ``[Image, metadata]`` where metadata holds screen size,
    cursor position, and the list of open windows.
    """
    capture_region = region
    offset_x = 0
    offset_y = 0

    if detect and region:
        enlarged_region = _enlarge_region(region)
        capture_region = enlarged_region
        offset_x = enlarged_region.x
        offset_y = enlarged_region.y
    elif region:
        offset_x = region.x
        offset_y = region.y

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

    if detect:
        eff_confidence = (
            confidence if confidence is not None
            else _adaptive_confidence(capture_region)
        )
        # Decode once and reuse for both detection and annotation drawing.
        pil_img = PILImage.open(io.BytesIO(raw_png)).convert("RGB")
        detections = await detect_elements(pil_img, confidence=eff_confidence)
        img_bytes = draw_detections(
            pil_img, detections, fmt=format, offset_x=offset_x, offset_y=offset_y,
        )
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
    img = PILImage.open(io.BytesIO(raw_png))
    return save_image_bytes(img, fmt)
