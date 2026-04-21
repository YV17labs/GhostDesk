# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Post-action visual feedback — poll the full screen until it changes."""

import asyncio
import io

from mcp.server.fastmcp import Context
from PIL import Image

from ghostdesk.screen._shared import capture_png, diff_against_rgb

# Capture scale used during feedback polling. Smaller = faster grim
# encode AND a natural downsample that filters out single-character
# clock ticks and blinking carets.
FEEDBACK_SCALE = 0.25

POLL_INTERVAL = 0.10
POLL_TIMEOUT = 2.0


async def capture_before() -> bytes:
    """Capture the full screen at reduced resolution before an action."""
    return await capture_png(scale=FEEDBACK_SCALE)


async def poll_for_change(
    before: bytes,
    timeout: float = POLL_TIMEOUT,
    interval: float = POLL_INTERVAL,
) -> dict:
    """Poll the full screen until it differs from ``before`` or timeout.

    Returns a dict with ``changed`` (bool) and ``reaction_time_ms`` (int).
    """
    before_rgb = Image.open(io.BytesIO(before))
    if before_rgb.mode != "RGB":
        before_rgb = before_rgb.convert("RGB")

    loop = asyncio.get_running_loop()
    start = loop.time()
    deadline = start + timeout
    while loop.time() < deadline:
        await asyncio.sleep(interval)
        now = await capture_png(scale=FEEDBACK_SCALE)
        if diff_against_rgb(before_rgb, now):
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


async def warn_on_miss(ctx: Context | None, feedback: dict) -> None:
    """Push an MCP warning when ``screen_changed`` is false."""
    if ctx is None or feedback.get("screen_changed", True):
        return
    await ctx.warning(
        f"{feedback['action']} — no visible screen change within {POLL_TIMEOUT:.0f}s"
    )
