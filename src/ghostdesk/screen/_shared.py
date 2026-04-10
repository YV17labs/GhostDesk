# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Shared utilities for screen capture and inspection."""

from __future__ import annotations

import asyncio
import io
import os
from dataclasses import dataclass

from PIL import Image, ImageChops, ImageDraw, ImageFont

SCREEN_WIDTH = int(os.environ.get("SCREEN_WIDTH", "1280"))
SCREEN_HEIGHT = int(os.environ.get("SCREEN_HEIGHT", "1024"))

# Stability detection: how long to wait for the screen to settle, the
# poll interval between samples, and the max ratio of changed area
# below which two consecutive captures are considered "stable enough"
# (ignores tiny diffs like a blinking cursor).
_STABILITY_TIMEOUT_S = 2.5
_STABILITY_POLL_S = 0.15
_STABILITY_MAX_DIFF_RATIO = 0.005


@dataclass
class Region:
    """Rectangular screen region to capture."""

    x: int
    y: int
    width: int
    height: int


async def capture_png(region: Region | None = None) -> bytes:
    """Capture the screen via maim and return raw PNG bytes.

    Waits for the screen to settle before returning, so the caller never
    sees a half-loaded UI. Two consecutive captures are taken; if they
    are nearly identical the first one is returned immediately. Otherwise
    we keep sampling until the screen is stable or `_STABILITY_TIMEOUT_S`
    elapses, in which case the latest capture is returned anyway.
    """
    prev = await _maim(region)
    deadline = asyncio.get_event_loop().time() + _STABILITY_TIMEOUT_S
    while True:
        curr = await _maim(region)
        if _screens_stable(prev, curr):
            return curr
        if asyncio.get_event_loop().time() >= deadline:
            return curr
        await asyncio.sleep(_STABILITY_POLL_S)
        prev = curr


async def _maim(region: Region | None) -> bytes:
    """Single maim invocation — no stability checks."""
    cmd = ["maim", "--format=png"]
    if region:
        cmd += ["-g", f"{region.width}x{region.height}+{region.x}+{region.y}"]
    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=10.0)
    if proc.returncode != 0:
        raise RuntimeError(stderr.decode().strip() or "maim failed")
    return stdout


def _screens_stable(a_bytes: bytes, b_bytes: bytes) -> bool:
    """True if two captures look the same up to a small tolerance.

    A small bounding box of differences (e.g. a blinking text cursor or
    a single animated icon) is treated as stable. A large diff (page
    still loading, scroll in progress) is not.
    """
    if a_bytes == b_bytes:
        return True
    a = Image.open(io.BytesIO(a_bytes))
    b = Image.open(io.BytesIO(b_bytes))
    if a.size != b.size:
        return False
    diff_bbox = ImageChops.difference(a, b).getbbox()
    if diff_bbox is None:
        return True
    dw = diff_bbox[2] - diff_bbox[0]
    dh = diff_bbox[3] - diff_bbox[1]
    diff_area = dw * dh
    total_area = a.width * a.height
    return (diff_area / total_area) < _STABILITY_MAX_DIFF_RATIO


_GRID_TOP_MARGIN = 12
_GRID_LEFT_MARGIN = 28
_GRID_MARGIN_BG = (255, 255, 255)
_GRID_X_STEP = 50  # vertical gridlines + X label spacing
_GRID_Y_STEP = 20  # horizontal gridlines + Y label spacing
# Alternating gridline colors. Drawn opaque (not alpha-blended) so we
# avoid a per-pixel composite pass on every screenshot — these are the
# pre-blended values that approximate the previous translucent look.
_GRID_LINE_COLORS = ((255, 100, 255), (100, 220, 255))
_GRID_TICK_COLOR = (0, 0, 0)
_GRID_LABEL_COLOR = (0, 0, 0)
_DEFAULT_FONT = ImageFont.load_default()


def draw_grid(
    img: Image.Image,
    origin: tuple[int, int] = (0, 0),
) -> Image.Image:
    """Add an X/Y coordinate ruler in margins around the image.

    The returned image is *larger* than the input: a top strip holds
    the X ruler and a left strip holds the Y ruler, so labels never
    cover the screenshot content. Thin alternating magenta/cyan
    gridlines overlay the content area — vertical lines every 50 px
    and horizontal lines every 20 px.

    Labels use absolute screen coordinates, so pass ``origin``
    (top-left of the captured region) when the image is a crop — the
    ruler values will match the real screen.

    Args:
        img: Source image. The returned image is a new RGB copy with
            margins added on the top and left edges.
        origin: ``(x, y)`` of the image's top-left in absolute screen
            coordinates. Defaults to ``(0, 0)``.

    Returns:
        A new RGB image of size ``(w + left_margin, h + top_margin)``.
    """
    src = img.convert("RGB")
    w, h = src.size
    ox, oy = origin
    top, left = _GRID_TOP_MARGIN, _GRID_LEFT_MARGIN

    canvas = Image.new("RGB", (w + left, h + top), _GRID_MARGIN_BG)
    canvas.paste(src, (left, top))
    draw = ImageDraw.Draw(canvas)
    font = _DEFAULT_FONT

    first_x = ox + ((-ox) % _GRID_X_STEP)
    for abs_x in range(first_x, ox + w, _GRID_X_STEP):
        lx = left + (abs_x - ox)
        color = _GRID_LINE_COLORS[(abs_x // _GRID_X_STEP) % 2]
        draw.line([(lx, top), (lx, top + h)], fill=color)
        draw.line([(lx, top - 2), (lx, top)], fill=_GRID_TICK_COLOR)
        txt = str(abs_x)
        x0, _, x1, _ = draw.textbbox((0, 0), txt, font=font)
        draw.text((lx - (x1 - x0) // 2, 0), txt, fill=_GRID_LABEL_COLOR, font=font)

    first_y = oy + ((-oy) % _GRID_Y_STEP)
    for abs_y in range(first_y, oy + h, _GRID_Y_STEP):
        ly = top + (abs_y - oy)
        color = _GRID_LINE_COLORS[(abs_y // _GRID_Y_STEP) % 2]
        draw.line([(left, ly), (left + w, ly)], fill=color)
        draw.line([(left - 2, ly), (left, ly)], fill=_GRID_TICK_COLOR)
        txt = str(abs_y)
        x0, _, x1, _ = draw.textbbox((0, 0), txt, font=font)
        # Vertical centering: subtract half the glyph height (~10 px).
        ty = max(top, ly - 5)
        draw.text((left - (x1 - x0) - 2, ty), txt, fill=_GRID_LABEL_COLOR, font=font)

    return canvas


def save_image_bytes(img: Image.Image, fmt: str = "png") -> bytes:
    """Encode PIL image to bytes in the requested format.

    Args:
        img: PIL Image object (should be RGB or RGBA).
        fmt: Output format ("png" or "webp").

    Returns:
        Image bytes in the requested format.
    """
    if fmt == "webp":
        img = img.convert("RGB")
    buf = io.BytesIO()
    if fmt == "webp":
        img.save(buf, format="WebP", method=4)
    else:
        img.save(buf, format="PNG")
    return buf.getvalue()


def build_metadata(
    cx: int, cy: int, windows: list[dict],
    region: Region | None = None,
) -> dict:
    """Build the standard metadata dict returned by screenshot()."""
    r = region or Region(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT)
    return {
        "screen": {"width": SCREEN_WIDTH, "height": SCREEN_HEIGHT},
        "region": {"x": r.x, "y": r.y, "width": r.width, "height": r.height},
        "cursor": {"x": cx, "y": cy},
        "windows": windows,
    }
