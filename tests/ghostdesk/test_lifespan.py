# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk._lifespan — Wayland warm-up + idle watcher wiring."""

import asyncio
from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk._lifespan import lifespan
from ghostdesk.server import create_app


async def test_lifespan_warms_up_wayland_on_enter():
    """Entering the lifespan binds the Wayland input singleton before yielding."""
    with patch("ghostdesk._lifespan.get_wayland_input", new_callable=AsyncMock) as mock_get, \
         patch.dict("os.environ", {"GHOSTDESK_IDLE_TIMEOUT": "0"}, clear=False):
        async with lifespan(object()) as ctx:
            mock_get.assert_awaited_once()
            assert ctx == {}


async def test_lifespan_propagates_warm_up_failure():
    """Warm-up failure propagates out of the lifespan."""
    class _BoomError(RuntimeError):
        pass

    with patch(
        "ghostdesk._lifespan.get_wayland_input",
        new_callable=AsyncMock,
        side_effect=_BoomError("missing zwlr_virtual_pointer_manager_v1"),
    ):
        with pytest.raises(_BoomError):
            async with lifespan(object()):
                pytest.fail("lifespan should not reach the yield")


async def test_create_app_wires_lifespan():
    """create_app() wires the lifespan into FastMCP."""
    app = create_app(port=9999)
    assert app.settings.lifespan is lifespan


async def test_lifespan_starts_idle_watcher_when_enabled():
    """A non-zero timeout schedules an idle_watcher task and cancels it on exit."""
    with patch("ghostdesk._lifespan.get_wayland_input", new_callable=AsyncMock), \
         patch("ghostdesk._lifespan.idle_watcher") as mock_watcher, \
         patch.dict("os.environ", {"GHOSTDESK_IDLE_TIMEOUT": "60"}, clear=False):
        # Make the watcher coroutine sleep until cancelled so the task stays alive
        # for the duration of the lifespan.
        async def _sleep_forever(*_a, **_kw):
            await asyncio.Event().wait()

        mock_watcher.side_effect = _sleep_forever

        async with lifespan(object()):
            mock_watcher.assert_called_once_with(60)


async def test_lifespan_skips_watcher_when_disabled():
    """timeout=0 disables the watcher entirely."""
    with patch("ghostdesk._lifespan.get_wayland_input", new_callable=AsyncMock), \
         patch("ghostdesk._lifespan.idle_watcher") as mock_watcher, \
         patch.dict("os.environ", {"GHOSTDESK_IDLE_TIMEOUT": "0"}, clear=False):
        async with lifespan(object()):
            mock_watcher.assert_not_called()


async def test_lifespan_uses_default_when_env_unset():
    """No env var → 30min default (1800s)."""
    import os

    env = {k: v for k, v in os.environ.items() if k != "GHOSTDESK_IDLE_TIMEOUT"}
    with patch("ghostdesk._lifespan.get_wayland_input", new_callable=AsyncMock), \
         patch("ghostdesk._lifespan.idle_watcher") as mock_watcher, \
         patch.dict("os.environ", env, clear=True):
        async def _sleep_forever(*_a, **_kw):
            await asyncio.Event().wait()
        mock_watcher.side_effect = _sleep_forever

        async with lifespan(object()):
            mock_watcher.assert_called_once_with(1800)


async def test_lifespan_cancels_watcher_on_shutdown():
    """The watcher task is cancelled when the lifespan exits."""
    with patch("ghostdesk._lifespan.get_wayland_input", new_callable=AsyncMock), \
         patch("ghostdesk._lifespan.idle_watcher") as mock_watcher, \
         patch.dict("os.environ", {"GHOSTDESK_IDLE_TIMEOUT": "60"}, clear=False):

        cancelled = asyncio.Event()

        async def _track_cancellation(*_a, **_kw):
            try:
                await asyncio.Event().wait()
            except asyncio.CancelledError:
                cancelled.set()
                raise

        mock_watcher.side_effect = _track_cancellation

        async with lifespan(object()):
            # Yield once so the watcher task starts and reaches its
            # ``await Event().wait()`` — otherwise cancel() lands before
            # the body has a chance to run, and the except branch never
            # executes.
            await asyncio.sleep(0)
            assert not cancelled.is_set()
        assert cancelled.is_set()
