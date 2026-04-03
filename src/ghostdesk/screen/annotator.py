# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Draw element annotations on screenshot images."""

from __future__ import annotations

import colorsys
import io

from PIL import Image, ImageDraw, ImageFont

from ghostdesk.screen.grounding import Element

_LABEL_TEXT = (255, 255, 255, 255)  # White label text

# Golden angle (~137.5°) gives maximum visual separation between
# consecutive hues — no two neighbors look alike.
_GOLDEN_ANGLE = 0.381966


def _generate_color(index: int) -> tuple[int, int, int]:
    """Generate a distinct color for element *index* using golden angle hue."""
    hue = (index * _GOLDEN_ANGLE) % 1.0
    r, g, b = colorsys.hsv_to_rgb(hue, 0.7, 0.9)
    return int(r * 255), int(g * 255), int(b * 255)


def annotate_image(
    png_bytes: bytes,
    elements: list[Element],
    *,
    output_format: str = "png",
    quality: int = 80,
) -> bytes:
    """Draw bounding boxes and coordinate labels on a screenshot."""
    img = Image.open(io.BytesIO(png_bytes)).convert("RGBA")
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)

    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 10)
    except (OSError, IOError):
        font = ImageFont.load_default()

    # All occupied zones: placed labels + all element bounding boxes.
    occupied: list[tuple[int, int, int, int]] = []
    for el in elements:
        occupied.append((el.x, el.y, el.x + el.width, el.y + el.height))

    for i, el in enumerate(elements):
        r, g, b = _generate_color(i)
        box_fill = (r, g, b, 100)
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

        # Coordinate label.
        coord_text = f"({el.center_x},{el.center_y})"
        bbox = font.getbbox(coord_text)
        tw = bbox[2] - bbox[0]
        th = bbox[3] - bbox[1]

        # Try multiple positions around the box, pick the one with
        # least overlap. Generates 8 base positions + 4 offset variants.
        lw = tw + 4
        lh = th + 2
        candidates = [
            (el.x, el.y - lh - 2),                        # top-left
            (el.x + el.width - lw, el.y - lh - 2),        # top-right
            (el.x, el.y + el.height + 2),                  # bottom-left
            (el.x + el.width - lw, el.y + el.height + 2),  # bottom-right
            (el.x + el.width + 2, el.y),                    # right-top
            (el.x + el.width + 2, el.y + el.height - lh),   # right-bottom
            (el.x - lw - 2, el.y),                          # left-top
            (el.x - lw - 2, el.y + el.height - lh),         # left-bottom
            # Offset variants (shifted further away).
            (el.x, el.y - lh - lh - 4),                    # far top-left
            (el.x, el.y + el.height + lh + 4),              # far bottom-left
            (el.x + el.width + lw + 4, el.y),               # far right
            (el.x - lw - lw - 4, el.y),                     # far left
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

        best_pos = (el.x, max(0, el.y - lh - 2))
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
    if output_format == "webp":
        result.save(buf, format="WebP", quality=quality, method=4)
    else:
        result.save(buf, format="PNG")
    return buf.getvalue()
