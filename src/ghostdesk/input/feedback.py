# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Post-action visual feedback — poll a screen zone until it changes."""

import asyncio
import hashlib

from ghostdesk.screen._shared import SCREEN_HEIGHT, SCREEN_WIDTH, Region, capture_png

# Zone size (pixels) captured around the action point.
ZONE_SIZE = 200

# Polling interval and maximum wait time (seconds).
POLL_INTERVAL = 0.10
POLL_TIMEOUT = 2.0


def _zone_region(x: int, y: int) -> Region:
    """Build a capture region centred on (x, y), clamped to screen bounds."""
    half = ZONE_SIZE // 2
    rx = max(0, min(x - half, SCREEN_WIDTH - ZONE_SIZE))
    ry = max(0, min(y - half, SCREEN_HEIGHT - ZONE_SIZE))
    rw = min(ZONE_SIZE, SCREEN_WIDTH - rx)
    rh = min(ZONE_SIZE, SCREEN_HEIGHT - ry)
    return Region(rx, ry, rw, rh)


def _png_hash(data: bytes) -> bytes:
    """Fast hash of raw PNG bytes."""
    return hashlib.md5(data).digest()


async def capture_before(x: int, y: int) -> tuple[Region, bytes]:
    """Capture the zone around (x, y) before an action.

    Returns the region used and the hash of the captured image.
    """
    region = _zone_region(x, y)
    png = await capture_png(region)
    return region, _png_hash(png)


async def poll_for_change(
    region: Region,
    before_hash: bytes,
    timeout: float = POLL_TIMEOUT,
    interval: float = POLL_INTERVAL,
) -> dict:
    """Poll the region until the image changes or timeout is reached.

    Returns a dict with ``changed`` (bool) and ``reaction_time_ms`` (int).
    """
    loop = asyncio.get_running_loop()
    start = loop.time()
    deadline = start + timeout
    while loop.time() < deadline:
        await asyncio.sleep(interval)
        current_hash = _png_hash(await capture_png(region))
        if current_hash != before_hash:
            elapsed_ms = int((loop.time() - start) * 1000)
            return {"changed": True, "reaction_time_ms": elapsed_ms}

    elapsed_ms = int((loop.time() - start) * 1000)
    return {"changed": False, "reaction_time_ms": elapsed_ms}


def build_feedback(action: str, poll_result: dict) -> dict:
    """Build the standard feedback dict returned by input tools."""
    return {
        "action": action,
        "screen_changed": poll_result["changed"],
        "reaction_time_ms": poll_result["reaction_time_ms"],
    }
