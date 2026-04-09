# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Draw UI element annotations on screenshots."""

from __future__ import annotations

import io
from typing import Any

from PIL import Image, ImageDraw, ImageFont

from ghostdesk.screen._shared import save_image_bytes

_FONT_PATH = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
_ANNOTATION_FONT_SIZE = 9
_COLORS = {
    "button": (0, 128, 255),      # Blue
    "input": (0, 200, 100),       # Green
    "text": (128, 0, 255),        # Purple
    "checkbox": (255, 128, 0),    # Orange
    "dropdown": (255, 0, 128),    # Pink
    "icon": (200, 200, 0),        # Yellow
    "image": (100, 100, 100),     # Gray
    "link": (0, 0, 200),          # Dark blue
}
_DEFAULT_COLOR = (100, 100, 100)  # Gray for unknown types


def _get_annotation_font() -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    """Return font for annotations (lazy init)."""
    try:
        return ImageFont.truetype(_FONT_PATH, _ANNOTATION_FONT_SIZE)
    except (OSError, IOError):
        return ImageFont.load_default()


def draw_detections(
    png_bytes: bytes,
    detections: list[dict[str, Any]],
    *,
    fmt: str = "png",
) -> bytes:
    """Draw detected UI elements on the screenshot.

    Draws colored bounding boxes and labels for each detected element.
    Box color depends on element type (button, input, etc.).
    Label shows element type and center coordinates.

    Args:
        png_bytes: PNG image bytes
        detections: List of detections from detect()
        fmt: Output format ("png" or "webp")

    Returns:
        PNG bytes with annotations drawn
    """
    img = Image.open(io.BytesIO(png_bytes)).convert("RGB")
    draw = ImageDraw.Draw(img)
    font = _get_annotation_font()

    for detection in detections:
        label = detection["label"]
        box = detection["box"]  # [x1, y1, x2, y2]
        center = detection["center"]  # [cx, cy]
        score = detection["score"]

        x1, y1, x2, y2 = box
        cx, cy = center

        # Get color for this element type
        color = _COLORS.get(label, _DEFAULT_COLOR)

        # Draw bounding box (2px thick)
        draw.rectangle([x1, y1, x2, y2], outline=color, width=2)

        # Draw label + coordinates at top-left of box, with background
        label_text = f"{label} ({cx},{cy})"
        bbox = font.getbbox(label_text)
        label_w = bbox[2] - bbox[0]
        label_h = bbox[3] - bbox[1]

        # Position label above box (or inside if near top edge)
        label_x = max(0, x1)
        label_y = max(0, y1 - label_h - 4)

        # Draw semi-transparent background for label
        bg_padding = 2
        bg_box = [
            label_x - bg_padding,
            label_y - bg_padding,
            label_x + label_w + bg_padding,
            label_y + label_h + bg_padding,
        ]
        draw.rectangle(bg_box, fill=color)

        # Draw label text in white
        draw.text((label_x, label_y), label_text, fill=(255, 255, 255), font=font)

        # Draw center point as a small crosshair
        crosshair_size = 4
        draw.line(
            [(cx - crosshair_size, cy), (cx + crosshair_size, cy)],
            fill=color,
            width=1,
        )
        draw.line(
            [(cx, cy - crosshair_size), (cx, cy + crosshair_size)],
            fill=color,
            width=1,
        )

    return save_image_bytes(img, fmt)
