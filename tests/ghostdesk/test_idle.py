# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk._idle — idle session watchdog."""

from __future__ import annotations

import asyncio
import json
import time
from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk import _idle
from ghostdesk._idle import (
    DEFAULT_IDLE_TIMEOUT_S,
    cleanup_views,
    idle_seconds,
    idle_watcher,
    mark_activity,
    parse_timeout,
)

MODULE = "ghostdesk._idle"


@pytest.fixture
def mock_swaymsg():
    """Patch ``ghostdesk._sway._swaymsg`` so tests don't touch the real socket.

    Each side_effect entry corresponds to one IPC call in order:
    typically the ``get_tree`` first, then one ``kill`` per view.
    Tests that previously stubbed ``_run_cmd`` with full ``swaymsg``
    argv lists are translated below to the trimmed argv that
    ``_swaymsg`` actually receives (``["-t", "get_tree"]``,
    ``["[con_id=N]", "kill"]``).
    """
    with patch("ghostdesk._sway._swaymsg", new_callable=AsyncMock) as mock:
        mock.return_value = ""
        yield mock


# ---------------------------------------------------------------------------
# parse_timeout — defaults, disabled, valid, invalid
# ---------------------------------------------------------------------------


def test_parse_timeout_unset_uses_default():
    assert parse_timeout(None) == DEFAULT_IDLE_TIMEOUT_S


def test_parse_timeout_empty_uses_default():
    assert parse_timeout("") == DEFAULT_IDLE_TIMEOUT_S
    assert parse_timeout("   ") == DEFAULT_IDLE_TIMEOUT_S


def test_parse_timeout_zero_disables():
    assert parse_timeout("0") == 0


def test_parse_timeout_positive_seconds():
    assert parse_timeout("60") == 60
    assert parse_timeout("3600") == 3600
    assert parse_timeout(" 120 ") == 120


def test_parse_timeout_negative_falls_back_to_default():
    assert parse_timeout("-1") == DEFAULT_IDLE_TIMEOUT_S


def test_parse_timeout_garbage_falls_back_to_default():
    assert parse_timeout("five minutes") == DEFAULT_IDLE_TIMEOUT_S
    assert parse_timeout("5m") == DEFAULT_IDLE_TIMEOUT_S
    assert parse_timeout("3.5") == DEFAULT_IDLE_TIMEOUT_S


def test_parse_timeout_default_is_thirty_minutes():
    """Contract: default is 30 minutes, even if the constant changes."""
    assert DEFAULT_IDLE_TIMEOUT_S == 1800


# ---------------------------------------------------------------------------
# mark_activity / idle_seconds
# ---------------------------------------------------------------------------


def test_mark_activity_resets_idle_clock():
    mark_activity()
    assert idle_seconds() < 0.1


def test_idle_seconds_grows_without_marks():
    """Manually move the stamp into the past and check the elapsed reading."""
    fake_now = 1000.0
    with patch(f"{MODULE}.time.monotonic", return_value=fake_now - 42.0):
        mark_activity()
    with patch(f"{MODULE}.time.monotonic", return_value=fake_now):
        assert idle_seconds() == pytest.approx(42.0)


# ---------------------------------------------------------------------------
# cleanup_views — tree walk + swaymsg invocations
# ---------------------------------------------------------------------------


def _tree_with(*views: dict) -> dict:
    """Build a minimal Sway tree containing the given view leaves."""
    return {
        "id": 1, "type": "root",
        "nodes": [{
            "id": 2, "type": "output", "name": "HEADLESS-1",
            "nodes": [{
                "id": 3, "type": "workspace", "name": "1",
                "nodes": list(views),
                "floating_nodes": [],
            }],
            "floating_nodes": [],
        }],
        "floating_nodes": [],
    }


async def test_cleanup_views_kills_each_pid_node(mock_swaymsg):
    tree = _tree_with(
        {"id": 10, "type": "con", "pid": 1234, "app_id": "firefox", "nodes": [], "floating_nodes": []},
        {"id": 11, "type": "con", "pid": 5678, "name": "foot", "nodes": [], "floating_nodes": []},
    )
    mock_swaymsg.side_effect = [json.dumps(tree), "", ""]

    closed = await cleanup_views()

    assert closed == 2
    calls = [c.args[0] for c in mock_swaymsg.call_args_list]
    assert calls[0] == ["-t", "get_tree"]
    # Kills run concurrently via asyncio.gather, so order isn't fixed.
    assert sorted(calls[1:]) == [["[con_id=10]", "kill"], ["[con_id=11]", "kill"]]


async def test_cleanup_views_skips_nodes_without_pid(mock_swaymsg):
    """Workspaces / outputs / scratchpad have no pid and must be spared."""
    tree = _tree_with(
        # A real view…
        {"id": 10, "type": "con", "pid": 99, "app_id": "mousepad", "nodes": [], "floating_nodes": []},
    )
    mock_swaymsg.side_effect = [json.dumps(tree), ""]

    closed = await cleanup_views()

    assert closed == 1
    # Only the real view got a kill; the output/workspace/root containers did not.
    kill_calls = [c for c in mock_swaymsg.call_args_list if c.args[0] and "kill" in c.args[0]]
    assert len(kill_calls) == 1
    assert kill_calls[0].args[0] == ["[con_id=10]", "kill"]


async def test_cleanup_views_walks_floating(mock_swaymsg):
    tree = {
        "id": 1, "type": "root",
        "nodes": [{
            "id": 2, "type": "output",
            "nodes": [{
                "id": 3, "type": "workspace",
                "nodes": [],
                "floating_nodes": [
                    {"id": 99, "type": "con", "pid": 7, "app_id": "galculator",
                     "nodes": [], "floating_nodes": []},
                ],
            }],
            "floating_nodes": [],
        }],
        "floating_nodes": [],
    }
    mock_swaymsg.side_effect = [json.dumps(tree), ""]

    closed = await cleanup_views()

    assert closed == 1
    kill_calls = [c for c in mock_swaymsg.call_args_list if c.args[0] and "kill" in c.args[0]]
    assert kill_calls[0].args[0] == ["[con_id=99]", "kill"]


async def test_cleanup_views_handles_get_tree_failure(mock_swaymsg):
    mock_swaymsg.side_effect = RuntimeError("swaymsg gone")
    assert await cleanup_views() == 0


async def test_cleanup_views_handles_malformed_json(mock_swaymsg):
    mock_swaymsg.return_value = "not json {"
    assert await cleanup_views() == 0


async def test_cleanup_views_continues_when_one_kill_fails(mock_swaymsg):
    """One client dying loud must not block the others from being closed."""
    tree = _tree_with(
        {"id": 10, "type": "con", "pid": 1, "app_id": "a", "nodes": [], "floating_nodes": []},
        {"id": 11, "type": "con", "pid": 2, "app_id": "b", "nodes": [], "floating_nodes": []},
    )
    # get_tree succeeds, first kill fails, second kill succeeds.
    mock_swaymsg.side_effect = [json.dumps(tree), RuntimeError("nope"), ""]

    closed = await cleanup_views()
    assert closed == 1


# ---------------------------------------------------------------------------
# idle_watcher — fires on threshold, no-op below threshold, cancellable
# ---------------------------------------------------------------------------


async def test_idle_watcher_disabled_returns_immediately():
    """timeout_s <= 0 must not start an infinite loop."""
    await asyncio.wait_for(idle_watcher(0), timeout=0.5)


async def test_idle_watcher_fires_cleanup_on_threshold():
    """When idle_seconds exceeds the threshold, cleanup runs and clock resets."""
    cleanup = AsyncMock(return_value=3)
    fired = asyncio.Event()

    async def _cleanup() -> int:
        n = await cleanup()
        fired.set()
        return n

    # Pre-age the activity stamp into the distant past so the very first
    # poll sees an idle window well above the threshold. We can't patch
    # time.monotonic globally — asyncio's event loop reads it too, and a
    # patched clock breaks asyncio.sleep / wait_for scheduling.
    _idle._last_activity_ts = time.monotonic() - 10_000.0

    task = asyncio.create_task(
        idle_watcher(50, cleanup_fn=_cleanup, poll_interval=0.01),
    )
    try:
        await asyncio.wait_for(fired.wait(), timeout=2.0)
    finally:
        task.cancel()
        try:
            await task
        except asyncio.CancelledError:
            pass

    cleanup.assert_awaited()


async def test_idle_watcher_skips_cleanup_when_active():
    """Activity inside the polling window keeps cleanup from firing."""
    cleanup = AsyncMock(return_value=0)

    async def _drive() -> None:
        for _ in range(5):
            await asyncio.sleep(0.005)
            mark_activity()

    task = asyncio.create_task(
        idle_watcher(60, cleanup_fn=cleanup, poll_interval=0.005),
    )
    driver = asyncio.create_task(_drive())
    await driver
    await asyncio.sleep(0.02)
    task.cancel()
    try:
        await task
    except asyncio.CancelledError:
        pass

    cleanup.assert_not_awaited()


async def test_idle_watcher_cancel_propagates():
    """Cancelling the task lets us await it cleanly."""
    task = asyncio.create_task(
        idle_watcher(3600, cleanup_fn=AsyncMock(return_value=0), poll_interval=0.01),
    )
    await asyncio.sleep(0.02)
    task.cancel()
    with pytest.raises(asyncio.CancelledError):
        await task


# ---------------------------------------------------------------------------
# poll_interval picker
# ---------------------------------------------------------------------------


def test_poll_interval_clamps_below_one_second():
    assert _idle._poll_interval(1) == 1.0


def test_poll_interval_clamps_above_sixty_seconds():
    assert _idle._poll_interval(3600) == 60.0


def test_poll_interval_scales_linearly_in_band():
    assert _idle._poll_interval(300) == 30.0
