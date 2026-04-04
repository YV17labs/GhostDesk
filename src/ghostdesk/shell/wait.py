# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Wait tool — pause before the next action."""

import asyncio


async def wait(seconds: float = 2.0) -> str:
    """Wait before taking the next action.

    Use after clicking a link or submitting a form to let the page load.

    Args:
        seconds: Duration to wait in seconds (default 2, max 10).
    """
    seconds = min(seconds, 10.0)
    await asyncio.sleep(seconds)
    return f"Waited {seconds}s"
