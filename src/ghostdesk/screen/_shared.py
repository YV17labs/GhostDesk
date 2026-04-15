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


async def capture_png(region: Region | None = None) -> bytes:
    """Single grim invocation — returns raw PNG bytes.

    grim writes to stdout when the output path is ``-``. Region geometry
    follows the standard Wayland format ``X,Y WxH``.
    """
    cmd = ["grim", "-t", "png"]
    if region:
        cmd += ["-g", f"{region.x},{region.y} {region.width}x{region.height}"]
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


def screens_stable(a_bytes: bytes, b_bytes: bytes) -> bool:
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


