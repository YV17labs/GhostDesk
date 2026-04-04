# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Screenshot capture tool."""

import asyncio
import io
import os
import tempfile
from dataclasses import dataclass
from typing import Literal

from mcp.server.fastmcp import Image
from PIL import Image as PILImage

from ghostdesk._cmd import run

ImageFormat = Literal["webp", "png"]


@dataclass
class Region:
    """Rectangular screen region to capture."""

    x: int
    y: int
    width: int
    height: int


async def screenshot(
    region: Region | None = None,
    annotate: bool = False,
    format: ImageFormat = "png",
) -> Image:
    """Capture the screen. Returns an image.

    Args:
        region: Optional region to capture. If omitted, the entire
            screen is captured.
        annotate: If True, overlay detected UI elements with bounding
            boxes and coordinate labels. Useful for debugging.
        format: Image format — "png" or "webp".
    """
    fd, path = tempfile.mkstemp(suffix=".png")
    os.close(fd)
    try:
        cmd = ["maim", "--format=png"]
        if region:
            cmd += ["-g", f"{region.width}x{region.height}+{region.x}+{region.y}"]
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
                format=format,
            )
            return Image(data=annotated, format=format)

        if format == "webp":
            img = PILImage.open(io.BytesIO(raw_png))
            buf = io.BytesIO()
            img.save(buf, format="WEBP")
            return Image(data=buf.getvalue(), format="webp")

        return Image(data=raw_png, format="png")
    finally:
        try:
            os.unlink(path)
        except FileNotFoundError:
            pass
