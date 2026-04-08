# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Draw coordinate rulers on screenshot images."""

from __future__ import annotations

import io

from PIL import Image, ImageDraw, ImageFont

from ghostdesk.screen._shared import save_image_bytes

_FONT_PATH = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
_FONT_SIZE = 11

_font_cache: ImageFont.FreeTypeFont | ImageFont.ImageFont | None = None


def _get_font() -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    """Return the shared font instance (lazy init)."""
    global _font_cache  # noqa: PLW0603
    if _font_cache is None:
        try:
            _font_cache = ImageFont.truetype(_FONT_PATH, _FONT_SIZE)
        except (OSError, IOError):
            _font_cache = ImageFont.load_default()
    return _font_cache


def draw_rulers(
    png_bytes: bytes,
    *,
    offset_x: int = 0,
    offset_y: int = 0,
    fmt: str = "png",
) -> bytes:
    """Draw coordinate rulers on the edges of a screenshot.

    Adds a horizontal ruler on top (X coordinates) and vertical ruler on the
    left (Y coordinates) with marks every 50 pixels. Coordinates shown are
    absolute screen coordinates (offset + image position).
    """
    img = Image.open(io.BytesIO(png_bytes)).convert("RGB")

    img_width, img_height = img.size
    ruler_left = 60
    ruler_top = 30

    new_width = img_width + ruler_left
    new_height = img_height + ruler_top
    canvas = Image.new("RGB", (new_width, new_height), (255, 255, 255))
    canvas.paste(img, (ruler_left, ruler_top))

    draw = ImageDraw.Draw(canvas)
    font = _get_font()

    draw.line([(ruler_left, ruler_top), (new_width, ruler_top)], fill=(0, 0, 0), width=1)
    draw.line([(ruler_left, ruler_top), (ruler_left, new_height)], fill=(0, 0, 0), width=1)

    x_abs = (offset_x // 50) * 50
    last_label_x = -1000

    while x_abs < offset_x + img_width:
        x_rel = x_abs - offset_x
        if 0 <= x_rel < img_width:
            draw.line([(ruler_left + x_rel, ruler_top - 10), (ruler_left + x_rel, ruler_top)],
                      fill=(0, 0, 0), width=1)

            label = str(x_abs)
            bbox = font.getbbox(label)
            label_w = bbox[2] - bbox[0]
            label_x = ruler_left + x_rel - label_w // 2
            if label_x > last_label_x + label_w + 5:
                draw.text((label_x, 2), label, fill=(0, 0, 0), font=font)
                last_label_x = label_x + label_w
        x_abs += 50

    y_abs = (offset_y // 50) * 50
    last_label_y = -1000

    while y_abs < offset_y + img_height:
        y_rel = y_abs - offset_y
        if 0 <= y_rel < img_height:
            draw.line([(ruler_left - 10, ruler_top + y_rel), (ruler_left, ruler_top + y_rel)],
                      fill=(0, 0, 0), width=1)

            label = str(y_abs)
            bbox = font.getbbox(label)
            label_h = bbox[3] - bbox[1]
            label_w = bbox[2] - bbox[0]
            label_y = ruler_top + y_rel - label_h // 2
            if label_y > last_label_y + label_h + 5:
                draw.text((5, label_y), label, fill=(0, 0, 0), font=font)
                last_label_y = label_y + label_h
        y_abs += 50

    return save_image_bytes(canvas, fmt)
