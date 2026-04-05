# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screen reader tool — structured text-only element detection."""

import asyncio
import os
import tempfile

from ghostdesk._cmd import run
from ghostdesk.input.humanizer import get_cursor_position
from ghostdesk.screen.grounding import detect_elements
from ghostdesk.screen.windows import get_open_windows


async def inspect(
    x: int | None = None,
    y: int | None = None,
    width: int | None = None,
    height: int | None = None,
) -> dict:
    """Read the screen contents as structured JSON. No image is returned.

    Detects only text elements via OCR and returns their label and click
    coordinates. Does not detect icons or images. Use mouse_click()
    with the reported x, y coordinates to interact.

    Args:
        x, y, width, height: Optional region to inspect. All four must
            be provided together to inspect a specific area. If omitted,
            the entire screen is inspected.
    """
    region = all(v is not None for v in (x, y, width, height))

    fd, path = tempfile.mkstemp(suffix=".png")
    os.close(fd)
    try:
        windows_task = asyncio.create_task(get_open_windows())

        cmd = ["maim", "--format=png"]
        if region:
            cmd += ["-g", f"{width}x{height}+{x}+{y}"]
        cmd.append(path)
        await run(cmd)

        with open(path, "rb") as f:
            raw_png = f.read()

        (cx, cy), windows, elements = await asyncio.gather(
            get_cursor_position(),
            windows_task,
            asyncio.to_thread(detect_elements, raw_png),
        )

        if region:
            cx -= x  # type: ignore[operator]
            cy -= y  # type: ignore[operator]
            for el in elements:
                el.x += x  # type: ignore[operator]
                el.y += y  # type: ignore[operator]
                el.center_x = el.x + el.width // 2
                el.center_y = el.y + el.height // 2

        result = {
            "cursor": {"x": cx, "y": cy},
            "windows": windows,
            "elements": [el.to_dict() for el in elements],
        }

        return result
    finally:
        try:
            os.unlink(path)
        except FileNotFoundError:
            pass
