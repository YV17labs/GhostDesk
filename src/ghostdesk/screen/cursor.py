# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Draw a visible cursor on screenshot images."""

import io
from typing import Literal

from PIL import Image, ImageDraw

ImageFormat = Literal["webp", "png"]


def draw_cursor(
    png_bytes: bytes,
    x: int,
    y: int,
    size: int = 24,
    color: tuple[int, int, int, int] = (255, 50, 50, 230),
    *,
    output_format: ImageFormat = "png",
    quality: int = 80,
) -> bytes:
    """Overlay a bright crosshair + dot on *png_bytes* at position (x, y).

    The crosshair is intentionally vivid so the LLM can always spot the
    current cursor position regardless of background.

    *output_format* controls the final encoding (``"webp"`` or ``"png"``).
    *quality* applies to WebP only (ignored for PNG).
    """
    if output_format not in ("webp", "png"):
        output_format = "png"
    quality = max(1, min(quality, 100))

    img = Image.open(io.BytesIO(png_bytes)).convert("RGBA")
    draw = ImageDraw.Draw(img)

    half = size // 2

    # Outer crosshair (dark border for contrast on light backgrounds)
    border = (0, 0, 0, 180)
    draw.line([(x - half, y), (x + half, y)], fill=border, width=4)
    draw.line([(x, y - half), (x, y + half)], fill=border, width=4)

    # Inner crosshair (bright color)
    draw.line([(x - half, y), (x + half, y)], fill=color, width=2)
    draw.line([(x, y - half), (x, y + half)], fill=color, width=2)

    # Center dot
    dot_r = 3
    draw.ellipse(
        [(x - dot_r, y - dot_r), (x + dot_r, y + dot_r)],
        fill=color,
    )

    result = img.convert("RGB")
    buf = io.BytesIO()
    if output_format == "webp":
        result.save(buf, format="WebP", quality=quality, method=4)
    else:
        result.save(buf, format="PNG")
    return buf.getvalue()
