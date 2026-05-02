# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""FastMCP lifespan — Wayland warm-up + idle-session watchdog.

Two boot-time concerns live here:

* Pre-bind the Wayland virtual pointer/keyboard so a missing compositor
  protocol surfaces at boot, not on the first ``mouse_click``.
* Arm the idle watchdog (see ``ghostdesk._idle``) so long-lived agents
  don't accumulate orphaned GUI windows.
"""

from __future__ import annotations

import asyncio
import logging
import os
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager

from mcp.server.fastmcp import FastMCP

from ghostdesk._idle import idle_watcher, mark_activity, parse_timeout
from ghostdesk.input._wayland import get_wayland_input

logger = logging.getLogger("ghostdesk")


@asynccontextmanager
async def lifespan(_server: FastMCP) -> AsyncIterator[dict]:
    """Warm up Wayland input and arm the idle-session watchdog."""
    logger.info("mcp-server: warming up Wayland input connection…")
    await get_wayland_input()
    logger.info("mcp-server: Wayland input ready (virtual pointer + keyboard bound)")

    timeout_s = parse_timeout(os.environ.get("GHOSTDESK_IDLE_TIMEOUT"))
    watcher_task: asyncio.Task | None = None

    if timeout_s > 0:
        # Reset the clock at boot so a slow startup doesn't immediately
        # look like an idle session and trigger cleanup before the agent
        # has had a chance to make a request.
        mark_activity()
        watcher_task = asyncio.create_task(
            idle_watcher(timeout_s), name="ghostdesk-idle-watcher",
        )
    else:
        logger.info(
            "mcp-server: idle watchdog disabled (GHOSTDESK_IDLE_TIMEOUT=0)",
        )

    try:
        yield {}
    finally:
        if watcher_task is not None:
            watcher_task.cancel()
            try:
                await watcher_task
            except asyncio.CancelledError:
                pass
            except Exception:
                logger.exception("mcp-server: idle watcher exited with error")
