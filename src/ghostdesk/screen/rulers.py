# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Draw coordinate rulers on screenshot images."""

from __future__ import annotations

import io

from PIL import Image, ImageDraw, ImageFont

from ghostdesk.screen._shared import save_image_bytes

_FONT_PATH = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
_FONT_SIZE = 11

_MINOR_STEP = 25
_MAJOR_STEP = 50
_MAJOR_TICK = 10
_MINOR_TICK = 5
_BLACK = (0, 0, 0)
_WHITE = (255, 255, 255)

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


def _draw_axis(
    draw: ImageDraw.ImageDraw,
    font: ImageFont.FreeTypeFont | ImageFont.ImageFont,
    *,
    axis: str,
    offset: int,
    length: int,
    ruler_left: int,
    ruler_top: int,
) -> None:
    """Draw ticks and labels along one axis. axis is 'x' or 'y'."""
    v = (offset // _MINOR_STEP) * _MINOR_STEP
    while v < offset + length:
        rel = v - offset
        if 0 <= rel < length:
            is_major = v % _MAJOR_STEP == 0
            tick_len = _MAJOR_TICK if is_major else _MINOR_TICK
            if axis == "x":
                px = ruler_left + rel
                draw.line([(px, ruler_top - tick_len), (px, ruler_top)], fill=_BLACK, width=1)
            else:
                py = ruler_top + rel
                draw.line([(ruler_left - tick_len, py), (ruler_left, py)], fill=_BLACK, width=1)
            if is_major:
                label = str(v)
                bbox = font.getbbox(label)
                label_w = bbox[2] - bbox[0]
                label_h = bbox[3] - bbox[1]
                if axis == "x":
                    draw.text(
                        (ruler_left + rel - label_w // 2, 2), label, fill=_BLACK, font=font
                    )
                else:
                    draw.text(
                        (5, ruler_top + rel - label_h // 2), label, fill=_BLACK, font=font
                    )
        v += _MINOR_STEP


def draw_rulers(
    png_bytes: bytes,
    *,
    offset_x: int = 0,
    offset_y: int = 0,
    fmt: str = "png",
) -> bytes:
    """Draw coordinate rulers on the edges of a screenshot.

    Adds a horizontal ruler on top (X coordinates) and vertical ruler on the
    left (Y coordinates) with major ticks + labels every 50 pixels and minor
    ticks every 25 pixels. Coordinates shown are absolute screen coordinates
    (offset + image position).
    """
    img = Image.open(io.BytesIO(png_bytes)).convert("RGB")

    img_width, img_height = img.size
    ruler_left = 60
    ruler_top = 30

    new_width = img_width + ruler_left
    new_height = img_height + ruler_top
    canvas = Image.new("RGB", (new_width, new_height), _WHITE)
    canvas.paste(img, (ruler_left, ruler_top))

    draw = ImageDraw.Draw(canvas)
    font = _get_font()

    draw.line([(ruler_left, ruler_top), (new_width, ruler_top)], fill=_BLACK, width=1)
    draw.line([(ruler_left, ruler_top), (ruler_left, new_height)], fill=_BLACK, width=1)

    _draw_axis(
        draw, font, axis="x", offset=offset_x, length=img_width,
        ruler_left=ruler_left, ruler_top=ruler_top,
    )
    _draw_axis(
        draw, font, axis="y", offset=offset_y, length=img_height,
        ruler_left=ruler_left, ruler_top=ruler_top,
    )

    return save_image_bytes(canvas, fmt)
