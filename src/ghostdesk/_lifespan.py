# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""FastMCP lifespan — pre-bind the Wayland virtual pointer/keyboard at startup.

Without this, a missing compositor protocol would only surface on the first
``mouse_click`` instead of at boot.
"""

from __future__ import annotations

import logging
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager

from mcp.server.fastmcp import FastMCP

from ghostdesk.input._wayland import get_wayland_input

logger = logging.getLogger("ghostdesk")


@asynccontextmanager
async def lifespan(_server: FastMCP) -> AsyncIterator[dict]:
    """Warm up the Wayland input singleton before accepting requests."""
    logger.info("mcp-server: warming up Wayland input connection…")
    await get_wayland_input()
    logger.info("mcp-server: Wayland input ready (virtual pointer + keyboard bound)")
    yield {}
