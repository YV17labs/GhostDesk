# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Draw bounding-box overlay on screenshot images."""

from __future__ import annotations

import colorsys
import io

from PIL import Image, ImageDraw, ImageFont

from ghostdesk.screen.grounding import Element

_LABEL_TEXT = (0, 0, 0, 255)  # Black label text
_FONT_PATH = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
_FONT_SIZE = 10

# Golden angle (~137.5°) gives maximum visual separation between
# consecutive hues — no two neighbors look alike.
_GOLDEN_ANGLE = 0.381966

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


def _generate_color(index: int) -> tuple[int, int, int]:
    """Generate a distinct color for element *index* using golden angle hue."""
    hue = (index * _GOLDEN_ANGLE) % 1.0
    r, g, b = colorsys.hsv_to_rgb(hue, 0.45, 0.95)
    return int(r * 255), int(g * 255), int(b * 255)


def draw_overlay(
    png_bytes: bytes,
    elements: list[Element],
    *,
    fmt: str = "png",
    offset: tuple[int, int] = (0, 0),
) -> bytes:
    """Draw bounding boxes and coordinate labels on a screenshot.

    Args:
        offset: (dx, dy) added to displayed coordinate labels so they
            show absolute screen positions even on cropped images.
    """
    img = Image.open(io.BytesIO(png_bytes)).convert("RGBA")
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)

    font = _get_font()

    # All occupied zones: placed labels + all element bounding boxes.
    occupied: list[tuple[int, int, int, int]] = []
    for el in elements:
        occupied.append((el.x, el.y, el.x + el.width, el.y + el.height))

    for i, el in enumerate(elements):
        r, g, b = _generate_color(i)
        box_fill = (r, g, b, 60)
        box_outline = (r, g, b, 255)
        label_bg = (r, g, b, 200)

        # Bounding box.
        draw.rectangle(
            [el.x, el.y, el.x + el.width, el.y + el.height],
            outline=box_outline,
            width=2,
        )
        draw.rectangle(
            [el.x + 1, el.y + 1, el.x + el.width - 1, el.y + el.height - 1],
            fill=box_fill,
        )

        dx, dy = offset
        coord_text = f"({el.center_x + dx},{el.center_y + dy})"
        bbox = font.getbbox(coord_text)
        tw = bbox[2] - bbox[0]
        th = bbox[3] - bbox[1]

        # Try multiple positions around the box, pick the one with
        # least overlap. 8 outside positions + 2 inside fallbacks.
        lw = tw + 4
        lh = th + 2
        candidates = [
            (el.x, el.y - lh),                              # top-left (outside)
            (el.x + el.width - lw, el.y - lh),              # top-right (outside)
            (el.x, el.y + el.height),                        # bottom-left (outside)
            (el.x + el.width - lw, el.y + el.height),        # bottom-right (outside)
            (el.x + el.width, el.y),                          # right-top
            (el.x + el.width, el.y + el.height - lh),         # right-bottom
            (el.x - lw, el.y),                                # left-top
            (el.x - lw, el.y + el.height - lh),               # left-bottom
            # Inside the box — guarantees visual association.
            (el.x, el.y),                                      # inside top-left
            (el.x + el.width - lw, el.y),                      # inside top-right
        ]

        # Own box index in occupied list — skip it when measuring overlap.
        own_idx = i

        def _overlap_area(cx: int, cy: int) -> int:
            """Total overlap area with all occupied zones (boxes + labels)."""
            area = 0
            for j, (px1, py1, px2, py2) in enumerate(occupied):
                if j == own_idx:
                    continue  # don't penalize overlap with own box
                ox = max(0, min(cx + lw, px2) - max(cx, px1))
                oy = max(0, min(cy + lh, py2) - max(cy, py1))
                area += ox * oy
            return area

        best_pos = (el.x, max(0, el.y - lh))
        best_score = float("inf")

        for cx, cy in candidates:
            # Clamp to image bounds.
            cx = max(0, min(cx, img.width - lw))
            cy = max(0, min(cy, img.height - lh))
            score = _overlap_area(cx, cy)
            if score == 0:
                best_pos = (cx, cy)
                break
            if score < best_score:
                best_score = score
                best_pos = (cx, cy)

        lx, ly = best_pos

        occupied.append((lx, ly, lx + lw, ly + lh))
        draw.rectangle([lx, ly, lx + lw, ly + lh], fill=label_bg)
        draw.text((lx + 2, ly), coord_text, fill=_LABEL_TEXT, font=font)

    result = Image.alpha_composite(img, overlay).convert("RGB")
    buf = io.BytesIO()
    if fmt == "webp":
        result.save(buf, format="WebP", method=4)
    else:
        result.save(buf, format="PNG")
    return buf.getvalue()
