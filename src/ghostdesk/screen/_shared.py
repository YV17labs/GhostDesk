# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Shared utilities for screen capture and inspection."""

from __future__ import annotations

import asyncio
import os
from dataclasses import dataclass

from ghostdesk.screen.grounding import Element

SCREEN_WIDTH = int(os.environ.get("SCREEN_WIDTH", "1280"))
SCREEN_HEIGHT = int(os.environ.get("SCREEN_HEIGHT", "1024"))


@dataclass
class Region:
    """Rectangular screen region to capture."""

    x: int
    y: int
    width: int
    height: int


async def capture_png(region: Region | None = None) -> bytes:
    """Capture the screen via maim and return raw PNG bytes."""
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


def apply_region_offset(elements: list[Element], region: Region) -> None:
    """Shift element coordinates from region-relative to absolute screen positions."""
    for el in elements:
        el.x += region.x
        el.y += region.y
        el.center_x = el.x + el.width // 2
        el.center_y = el.y + el.height // 2


def build_metadata(
    cx: int, cy: int, windows: list[dict], elements: list[Element],
) -> dict:
    """Build the standard metadata dict returned by screenshot() and inspect()."""
    return {
        "screen": {"width": SCREEN_WIDTH, "height": SCREEN_HEIGHT},
        "cursor": {"x": cx, "y": cy},
        "windows": windows,
        "elements": [el.to_dict() for el in elements],
    }
