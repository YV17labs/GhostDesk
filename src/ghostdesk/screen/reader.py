# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screen reader tool — structured text-only element detection."""

import asyncio

from ghostdesk.input.humanizer import get_cursor_position
from ghostdesk.screen._shared import Region, apply_region_offset, build_metadata, capture_png
from ghostdesk.screen.grounding import detect_elements
from ghostdesk.screen.windows import get_open_windows


async def inspect(
    region: Region | None = None,
) -> dict:
    """Read the screen contents as structured JSON. No image is returned.

    Detects only text elements via OCR and returns their label and click
    coordinates. Does not detect icons or images. Use mouse_click()
    with the reported x, y coordinates to interact.

    Args:
        region: Optional region to inspect. If omitted, the entire
            screen is inspected. Coordinates are always absolute
            screen positions, even when a region is used.
    """
    windows_task = asyncio.create_task(get_open_windows())
    cursor_task = asyncio.create_task(get_cursor_position())

    raw_png = await capture_png(region)

    (cx, cy), windows, elements = await asyncio.gather(
        cursor_task,
        windows_task,
        asyncio.to_thread(detect_elements, raw_png),
    )

    if region:
        apply_region_offset(elements, region)

    return build_metadata(cx, cy, windows, elements, region)
