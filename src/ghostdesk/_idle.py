# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Idle session watchdog — close all Sway views after N seconds of MCP silence.

Long-running agents tend to leak GUI apps: every Firefox tab, every
``foot`` shell, every ``mousepad`` window stays resident until the
container is killed. This module wires a server-side timer that, when
no MCP tool call has been observed for ``GHOSTDESK_IDLE_TIMEOUT``
seconds, walks the Sway tree and asks every client window to close.

Configuration is operator-side, **not** client-side. The agent cannot
override this; it is a property of the deployment.

The value is always expressed in **seconds** — no unit suffix.

* Unset / invalid → 30 minute default (1800s)
* ``0`` → watchdog disabled (no cleanup ever)
* Positive integer → seconds before idle cleanup fires.

The watchdog spares the desktop infrastructure (sway, mako, wayvnc,
supervisord, the MCP server itself): those processes are not Sway
*views*, so walking the tree's view nodes never reaches them.
"""

from __future__ import annotations

import asyncio
import logging
import time
from collections.abc import Awaitable, Callable

from ghostdesk._sway import collect_views, get_tree, kill_view

logger = logging.getLogger("ghostdesk")

DEFAULT_IDLE_TIMEOUT_S = 1800  # 30 minutes

# Initialise at import so the first poll never sees 0 (which would look
# like "idle since the epoch" and trigger an instant kill).
_last_activity_ts: float = time.monotonic()


def parse_timeout(raw: str | None) -> int:
    """Parse the ``GHOSTDESK_IDLE_TIMEOUT`` env var into a seconds count.

    The value is a bare integer expressed in seconds. ``0`` disables the
    watchdog. None, empty, negative, or unparseable input falls back to
    ``DEFAULT_IDLE_TIMEOUT_S`` — we never want a typo to silently disable
    the safety net.
    """
    if raw is None:
        return DEFAULT_IDLE_TIMEOUT_S
    s = raw.strip()
    if not s:
        return DEFAULT_IDLE_TIMEOUT_S

    try:
        n = int(s)
    except ValueError:
        logger.warning(
            "idle-watcher: could not parse GHOSTDESK_IDLE_TIMEOUT=%r, "
            "falling back to default %ds",
            raw, DEFAULT_IDLE_TIMEOUT_S,
        )
        return DEFAULT_IDLE_TIMEOUT_S

    if n < 0:
        logger.warning(
            "idle-watcher: negative GHOSTDESK_IDLE_TIMEOUT=%r, "
            "falling back to default %ds",
            raw, DEFAULT_IDLE_TIMEOUT_S,
        )
        return DEFAULT_IDLE_TIMEOUT_S

    return n


def mark_activity() -> None:
    """Reset the idle clock. Called on every MCP tool invocation."""
    global _last_activity_ts
    _last_activity_ts = time.monotonic()


def idle_seconds() -> float:
    """Return seconds elapsed since the last ``mark_activity()`` call."""
    return time.monotonic() - _last_activity_ts


async def _kill_one(con_id: int, label: str) -> bool:
    try:
        await kill_view(con_id)
    except Exception:
        logger.exception(
            "idle-cleanup: failed to close con_id=%d (%s)", con_id, label,
        )
        return False
    logger.info("idle-cleanup: closed view con_id=%d (%s)", con_id, label)
    return True


def _view_label(node: dict) -> str:
    return str(
        node.get("app_id")
        or node.get("name")
        or (node.get("window_properties") or {}).get("class")
        or "?"
    )


async def cleanup_views() -> int:
    """Close every Sway client window. Return the count of views closed.

    Walks ``swaymsg -t get_tree`` and sends a ``kill`` IPC command to
    each node that has a ``pid`` (i.e. real client surfaces — workspaces,
    outputs and the scratchpad have no pid). Layer-shell clients (mako)
    are not part of the regular tree and are spared.

    Each kill is a graceful close request: the client receives a Wayland
    close event and may flush state before exiting. Kills run
    concurrently so a slow-closing client cannot delay the rest.
    """
    tree = await get_tree()
    if tree is None:
        return 0

    targets = collect_views(tree, lambda n: (n["id"], _view_label(n)))
    if not targets:
        return 0

    results = await asyncio.gather(
        *(_kill_one(con_id, label) for con_id, label in targets),
    )
    return sum(results)


def _poll_interval(timeout_s: int) -> float:
    """Pick a polling cadence: 1/10th of timeout, clamped to [1s, 60s].

    Short enough that the timer fires within ~10% of the configured
    threshold; long enough that we don't burn CPU on a 1-hour deadline.
    """
    return max(1.0, min(60.0, timeout_s / 10))


async def idle_watcher(
    timeout_s: int,
    cleanup_fn: Callable[[], Awaitable[int]] = cleanup_views,
    poll_interval: float | None = None,
) -> None:
    """Forever-loop: when idle for ``timeout_s``, run cleanup and reset the clock.

    Cancelled by the lifespan on shutdown via ``task.cancel()``.
    """
    if timeout_s <= 0:
        # Defensive: lifespan filters this out, but stay correct if called directly.
        return

    interval = poll_interval if poll_interval is not None else _poll_interval(timeout_s)
    logger.info(
        "idle-watcher: armed (timeout=%ds, poll=%.1fs)", timeout_s, interval,
    )

    while True:
        try:
            await asyncio.sleep(interval)
        except asyncio.CancelledError:
            logger.info("idle-watcher: cancelled")
            raise

        elapsed = idle_seconds()
        if elapsed < timeout_s:
            continue

        logger.info(
            "idle-watcher: %.0fs idle ≥ %ds threshold — closing all views",
            elapsed, timeout_s,
        )
        try:
            closed = await cleanup_fn()
        except asyncio.CancelledError:
            raise
        except Exception:
            logger.exception("idle-watcher: cleanup failed")
            closed = 0

        logger.info("idle-watcher: cleanup done (%d views closed)", closed)
        mark_activity()
