# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screenshot capture tool."""

import asyncio
import os
import tempfile

from mcp.server.fastmcp import Image

from ghostdesk._cmd import run
from ghostdesk.screen.cursor import ImageFormat, draw_cursor
from ghostdesk.input.humanizer import get_cursor_position


async def screenshot(
    x: int | None = None,
    y: int | None = None,
    width: int | None = None,
    height: int | None = None,
    output_format: ImageFormat = "png",
    quality: int = 80,
    annotate: bool = False,
) -> Image:
    """Capture the screen. Returns an image.

    Args:
        x, y, width, height: Optional region to capture. All four must
            be provided together to capture a specific area. If omitted,
            the entire screen is captured.
        output_format: Image format — "png" or "webp".
        quality: WebP quality (1-100). Ignored for PNG.
        annotate: If True, overlay detected UI elements with bounding
            boxes and coordinate labels. Useful for debugging.
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

        if annotate:
            from ghostdesk.screen.annotator import annotate_image
            from ghostdesk.screen.grounding import detect_elements

            elements = await asyncio.to_thread(detect_elements, raw_png)
            annotated = annotate_image(
                raw_png, elements,
                output_format=output_format, quality=quality,
            )
            return Image(data=annotated, format=output_format)

        cx, cy = await get_cursor_position()

        if region:
            cx -= x  # type: ignore[operator]
            cy -= y  # type: ignore[operator]

        image_bytes = draw_cursor(
            raw_png, cx, cy,
            output_format=output_format,
            quality=quality,
        )

        return Image(data=image_bytes, format=output_format)
    finally:
        try:
            os.unlink(path)
        except FileNotFoundError:
            pass
