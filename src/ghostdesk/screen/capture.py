# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screenshot capture tool."""

import asyncio
import io
from typing import Literal

from mcp.server.fastmcp import Image
from PIL import Image as PILImage

from ghostdesk._cursor import get_cursor_position
from ghostdesk.screen._shared import Region, apply_region_offset, build_metadata, capture_png
from ghostdesk.screen.windows import get_open_windows

ImageFormat = Literal["webp", "png"]


async def screenshot(
    region: Region | None = None,
    overlay: bool = False,
    format: ImageFormat = "png",
) -> list:
    """Capture the screen. Returns an image and detected UI elements.

    Args:
        region: Optional region to capture. If omitted, the entire
            screen is captured.
        overlay: If True, draw bounding boxes and coordinate labels
            on the image for visual reference.
        format: Image format — "png" or "webp".

    Returns a list containing:
        - The screenshot image (with or without visual overlay).
        - A JSON object with screen dimensions, captured region,
          cursor position, open windows, and all detected UI elements
          with absolute screen coordinates — use these with mouse_click().
    """
    from ghostdesk.screen.grounding import detect_elements

    cursor_task = asyncio.create_task(get_cursor_position())
    windows_task = asyncio.create_task(get_open_windows())

    raw_png = await capture_png(region)

    (cx, cy), windows, elements = await asyncio.gather(
        cursor_task,
        windows_task,
        asyncio.to_thread(detect_elements, raw_png),
    )

    if overlay:
        from ghostdesk.screen.overlay import draw_overlay

        label_offset = (region.x, region.y) if region else (0, 0)
        img_bytes = draw_overlay(
            raw_png, elements, fmt=format, offset=label_offset,
        )

    if region:
        apply_region_offset(elements, region)

    metadata = build_metadata(cx, cy, windows, elements, region)

    if overlay:
        return [Image(data=img_bytes, format=format), metadata]

    if format == "webp":
        img = PILImage.open(io.BytesIO(raw_png))
        buf = io.BytesIO()
        img.save(buf, format="WEBP")
        return [Image(data=buf.getvalue(), format="webp"), metadata]

    return [Image(data=raw_png, format="png"), metadata]
