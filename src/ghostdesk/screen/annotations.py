# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Draw UI element annotations on screenshots."""

from __future__ import annotations

from typing import Any

from PIL import Image, ImageDraw, ImageFont

from ghostdesk.screen._shared import save_image_bytes

_FONT_PATH = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
_ANNOTATION_FONT_SIZE = 11
_LABEL_BG_PADDING = 2
_CROSSHAIR_HALF_SIZE = 4
_BOX_OUTLINE_WIDTH = 2

# Per-detection palette: each box+label pair gets a distinct flashy color so
# that displaced labels can still be visually associated with their box.
# Hand-picked for high luminance so black text stays readable.
_FLASHY_PALETTE = [
    (255, 128, 0),    # orange
    (0, 200, 255),    # sky blue
    (0, 220, 80),     # bright green
    (255, 80, 80),    # coral
    (255, 0, 200),    # hot pink
    (200, 100, 255),  # lavender
    (255, 220, 0),    # gold
    (100, 255, 200),  # mint
    (255, 160, 100),  # peach
    (180, 220, 80),   # lime
]
_LABEL_TEXT_COLOR = (0, 0, 0)
# Slight transparency on label backgrounds so the underlying pixels remain
# faintly visible without harming text readability. 0=invisible, 255=opaque.
_LABEL_BG_ALPHA = 215


def _get_annotation_font() -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    """Return font for annotations (lazy init)."""
    try:
        return ImageFont.truetype(_FONT_PATH, _ANNOTATION_FONT_SIZE)
    except (OSError, IOError):
        return ImageFont.load_default()


def _rects_overlap(
    a: tuple[int, int, int, int], b: tuple[int, int, int, int],
) -> bool:
    """Return True if two (x1, y1, x2, y2) rectangles intersect."""
    return not (a[2] <= b[0] or b[2] <= a[0] or a[3] <= b[1] or b[3] <= a[1])


def _find_label_position(
    box: tuple[int, int, int, int],
    label_w: int,
    label_h: int,
    bg_padding: int,
    img_w: int,
    img_h: int,
    placed: list[tuple[int, int, int, int]],
) -> tuple[int, int]:
    """Find a non-overlapping position for a label near its box.

    Tries a list of candidate anchor points around the box (above, below,
    inside, sides). Returns the first candidate whose background rectangle
    fits inside the image and does not collide with any already-placed
    label rectangle. Falls back to the default "above-left" position if
    every candidate collides.
    """
    x1, y1, x2, y2 = box
    # Labels are placed OUTSIDE the box. Both the perpendicular edge AND the
    # parallel edge of the background rectangle are aligned with the box's
    # edges (e.g. above-left = bg.bottom touches box.top AND bg.left touches
    # box.left). All ±bg_padding shifts compensate for the bg expansion.
    candidates = [
        (x1 + bg_padding,           y1 - label_h - bg_padding),
        (x2 - label_w - bg_padding, y1 - label_h - bg_padding),
        (x1 + bg_padding,           y2 + bg_padding),
        (x2 - label_w - bg_padding, y2 + bg_padding),
        (x2 + bg_padding,           y1 + bg_padding),
        (x1 - label_w - bg_padding, y1 + bg_padding),
        (x2 + bg_padding,           y2 - label_h - bg_padding),
        (x1 - label_w - bg_padding, y2 - label_h - bg_padding),
    ]

    for lx, ly in candidates:
        bg = (
            lx - bg_padding,
            ly - bg_padding,
            lx + label_w + bg_padding,
            ly + label_h + bg_padding,
        )
        if bg[0] < 0 or bg[1] < 0 or bg[2] > img_w or bg[3] > img_h:
            continue
        if any(_rects_overlap(bg, other) for other in placed):
            continue
        return lx, ly

    fallback_x = max(bg_padding, min(x1 + bg_padding, img_w - label_w - bg_padding))
    fallback_y = max(bg_padding, y1 - label_h - bg_padding)
    return fallback_x, fallback_y


def draw_detections(
    image: Image.Image,
    detections: list[dict[str, Any]],
    *,
    fmt: str = "png",
    offset_x: int = 0,
    offset_y: int = 0,
) -> bytes:
    """Draw detected UI elements on the screenshot.

    Each detection becomes a colored bounding box, a center crosshair, and a
    label rectangle showing its center point in absolute screen coordinates.
    Labels are placed outside the box with collision avoidance.

    Detections are in image-local coordinates; offset_x/offset_y are the
    top-left of the captured region in screen space, used only to compute
    the absolute screen coordinates displayed in label text (so the LLM can
    feed them straight into ``mouse_click``).

    Args:
        image: PIL Image of the captured area (will be converted to RGBA).
        detections: List of detections from detect() in image-local coords.
        fmt: Output format ("png" or "webp")
        offset_x: X offset of the captured region in absolute screen coordinates
        offset_y: Y offset of the captured region in absolute screen coordinates

    Returns:
        Encoded image bytes with annotations drawn.
    """
    img = image.convert("RGBA") if image.mode != "RGBA" else image.copy()
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    overlay_draw = ImageDraw.Draw(overlay)
    font = _get_annotation_font()
    img_w, img_h = img.size

    # Sort by box top so the color cycle and label placement are stable
    # top-down across runs.
    items = []
    for idx, detection in enumerate(
        sorted(detections, key=lambda d: d["box"][1]),
    ):
        cx, cy = detection["center"]
        x1, y1, x2, y2 = detection["box"]
        # Labels show ABSOLUTE screen coordinates so the LLM can copy them
        # straight into mouse_click; boxes are drawn in image-local coords.
        label_text = f"{cx + offset_x},{cy + offset_y}"
        bbox = font.getbbox(label_text)
        label_w = bbox[2] - bbox[0]
        label_h = bbox[3] - bbox[1]
        items.append({
            "box": (x1, y1, x2, y2),
            "local_center": (cx, cy),
            "label_text": label_text,
            "label_w": label_w,
            "label_h": label_h,
            "color": _FLASHY_PALETTE[idx % len(_FLASHY_PALETTE)],
        })

    placed: list[tuple[int, int, int, int]] = []
    label_drawings: list[tuple[int, int, str]] = []
    for item in items:
        color_opaque = item["color"] + (255,)
        color_alpha = item["color"] + (_LABEL_BG_ALPHA,)

        x1, y1, x2, y2 = item["box"]
        draw.rectangle([x1, y1, x2, y2], outline=color_opaque, width=_BOX_OUTLINE_WIDTH)
        lcx, lcy = item["local_center"]
        draw.line(
            [(lcx - _CROSSHAIR_HALF_SIZE, lcy), (lcx + _CROSSHAIR_HALF_SIZE, lcy)],
            fill=color_opaque, width=1,
        )
        draw.line(
            [(lcx, lcy - _CROSSHAIR_HALF_SIZE), (lcx, lcy + _CROSSHAIR_HALF_SIZE)],
            fill=color_opaque, width=1,
        )

        label_x, label_y = _find_label_position(
            item["box"],
            item["label_w"],
            item["label_h"],
            _LABEL_BG_PADDING,
            img_w,
            img_h,
            placed,
        )
        bg_box = (
            label_x - _LABEL_BG_PADDING,
            label_y - _LABEL_BG_PADDING,
            label_x + item["label_w"] + _LABEL_BG_PADDING,
            label_y + item["label_h"] + _LABEL_BG_PADDING,
        )
        overlay_draw.rectangle(list(bg_box), fill=color_alpha)
        placed.append(bg_box)
        label_drawings.append((label_x, label_y, item["label_text"]))

    # Compositing replaces img; text must be drawn on the composited result
    # so it stays fully opaque on top of the alpha-blended label backgrounds.
    img = Image.alpha_composite(img, overlay)
    final_draw = ImageDraw.Draw(img)
    for label_x, label_y, label_text in label_drawings:
        final_draw.text(
            (label_x, label_y), label_text,
            fill=_LABEL_TEXT_COLOR + (255,), font=font,
        )

    return save_image_bytes(img, fmt)
