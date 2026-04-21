# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Shared utilities for screen capture and inspection."""

from __future__ import annotations

import asyncio
import io
from dataclasses import dataclass

from PIL import Image, ImageChops

from ghostdesk._coords import SCREEN_HEIGHT, SCREEN_WIDTH

# Stability detection: max ratio of changed area below which two
# consecutive captures are considered "stable enough" (ignores tiny
# diffs like a blinking cursor).
_STABILITY_MAX_DIFF_RATIO = 0.005


@dataclass
class Region:
    """Rectangular screen region to capture."""

    x: int
    y: int
    width: int
    height: int


async def capture_png(
    region: Region | None = None,
    scale: float | None = None,
) -> bytes:
    """Single grim invocation — returns raw PNG bytes.

    grim writes to stdout when the output path is ``-``. Region geometry
    follows the standard Wayland format ``X,Y WxH``. ``scale`` (when set
    < 1.0) downsamples the encoded image — useful for cheap
    comparison-only captures where pixel fidelity does not matter.
    """
    cmd = ["grim", "-t", "png"]
    if region:
        cmd += ["-g", f"{region.x},{region.y} {region.width}x{region.height}"]
    if scale is not None:
        cmd += ["-s", f"{scale}"]
    cmd += ["-"]
    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=10.0)
    if proc.returncode != 0:
        raise RuntimeError(stderr.decode().strip() or "grim failed")
    return stdout


def _as_rgb(img: Image.Image) -> Image.Image:
    """Return ``img`` as RGB. ImageChops.difference on RGBA leaves alpha at 0
    (alpha is constant 255 on captures), and Pillow's ``getbbox()`` then
    ignores the whole diff because alpha 0 reads as transparent/empty."""
    return img if img.mode == "RGB" else img.convert("RGB")


def _bbox_ratio(a: Image.Image, b: Image.Image) -> float | None:
    """Bbox-of-difference area divided by image area, or ``None`` on size
    mismatch. Inputs must already be RGB."""
    if a.size != b.size:
        return None
    diff_bbox = ImageChops.difference(a, b).getbbox()
    if diff_bbox is None:
        return 0.0
    dw = diff_bbox[2] - diff_bbox[0]
    dh = diff_bbox[3] - diff_bbox[1]
    return (dw * dh) / (a.width * a.height)


def screens_differ(a_bytes: bytes, b_bytes: bytes) -> bool:
    """True if two captures differ by more than the stability threshold.

    Ignores tiny diffs (clock tick, blinking caret) and flags real
    changes (a popup opened, a row of cells got filled). Treats a size
    mismatch as a difference.
    """
    if a_bytes == b_bytes:
        return False
    a = _as_rgb(Image.open(io.BytesIO(a_bytes)))
    b = _as_rgb(Image.open(io.BytesIO(b_bytes)))
    ratio = _bbox_ratio(a, b)
    return ratio is None or ratio >= _STABILITY_MAX_DIFF_RATIO


def screens_stable(a_bytes: bytes, b_bytes: bytes) -> bool:
    """True if two captures look the same up to the stability threshold.

    Inverse of ``screens_differ`` — kept as a dedicated name because
    callers in the screen-stabilisation path read more naturally with
    the positive form.
    """
    return not screens_differ(a_bytes, b_bytes)


def diff_against_rgb(before_rgb: Image.Image, b_bytes: bytes) -> bool:
    """``screens_differ`` variant that takes a pre-decoded RGB Image as the
    baseline — useful in polling loops where ``before`` is fixed and we'd
    otherwise re-decode it on every iteration."""
    b = _as_rgb(Image.open(io.BytesIO(b_bytes)))
    ratio = _bbox_ratio(before_rgb, b)
    return ratio is None or ratio >= _STABILITY_MAX_DIFF_RATIO


def save_image_bytes(img: Image.Image, fmt: str = "png") -> bytes:
    """Encode PIL image to bytes in the requested format."""
    if fmt == "webp":
        img = img.convert("RGB")
    buf = io.BytesIO()
    if fmt == "webp":
        img.save(buf, format="WebP", method=4)
    else:
        img.save(buf, format="PNG")
    return buf.getvalue()


