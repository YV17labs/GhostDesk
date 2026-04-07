# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.input.mouse."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.input.mouse import (
    mouse_click,
    mouse_double_click,
    mouse_drag,
    mouse_scroll,
)

MODULE = "ghostdesk.input.mouse"

# Stub feedback so mouse tests stay focused on xdotool calls.
_FEEDBACK_RESULT = {"changed": True, "reaction_time_ms": 150}


@pytest.fixture(autouse=True)
def _mock_deps():
    with (
        patch(f"{MODULE}.run", new_callable=AsyncMock) as mock_run,
        patch(f"{MODULE}.get_cursor_position", new_callable=AsyncMock, return_value=(10, 20)) as mock_pos,
        patch(f"{MODULE}.human_move", new_callable=AsyncMock) as mock_hmove,
        patch(f"{MODULE}.capture_before", new_callable=AsyncMock, return_value=(None, b"h")) as mock_cap,
        patch(f"{MODULE}.poll_for_change", new_callable=AsyncMock, return_value=_FEEDBACK_RESULT) as mock_poll,
    ):
        yield mock_run, mock_pos, mock_hmove, mock_cap, mock_poll


def _mocks(_mock_deps):
    return _mock_deps


# --- mouse_click ---

async def test_mouse_click_humanize(_mock_deps):
    mock_run, _, mock_hmove, mock_cap, _ = _mock_deps
    result = await mouse_click(50, 60, button="left", humanize=True)
    mock_hmove.assert_awaited_once()
    # capture_before is called after move, before click
    mock_cap.assert_awaited_once_with(50, 60)
    mock_run.assert_awaited_once_with(["xdotool", "click", "1"])
    assert result["action"] == "Clicked left at (50, 60)"
    assert result["screen_changed"] is True


async def test_mouse_click_no_humanize(_mock_deps):
    mock_run, _, _, _, _ = _mock_deps
    result = await mouse_click(50, 60, button="right", humanize=False)
    assert mock_run.await_count == 2
    mock_run.assert_any_await(["xdotool", "mousemove", "50", "60"])
    mock_run.assert_any_await(["xdotool", "click", "3"])
    assert result["action"] == "Clicked right at (50, 60)"


async def test_mouse_click_middle_button(_mock_deps):
    mock_run, _, _, _, _ = _mock_deps
    await mouse_click(1, 2, button="middle", humanize=False)
    mock_run.assert_any_await(["xdotool", "click", "2"])


async def test_mouse_click_unknown_button_defaults_to_1(_mock_deps):
    mock_run, _, _, _, _ = _mock_deps
    await mouse_click(1, 2, button="unknown", humanize=False)
    mock_run.assert_any_await(["xdotool", "click", "1"])


async def test_mouse_click_no_change(_mock_deps):
    _, _, _, _, mock_poll = _mock_deps
    mock_poll.return_value = {"changed": False, "reaction_time_ms": 2000}
    result = await mouse_click(50, 60)
    assert result["screen_changed"] is False
    assert result["reaction_time_ms"] == 2000


# --- mouse_double_click ---

async def test_mouse_double_click_humanize(_mock_deps):
    mock_run, _, mock_hmove, _, _ = _mock_deps
    result = await mouse_double_click(30, 40, button="left", humanize=True)
    mock_hmove.assert_awaited_once()
    mock_run.assert_awaited_once_with(["xdotool", "click", "--repeat", "2", "--delay", "100", "1"])
    assert result["action"] == "Double-clicked left at (30, 40)"
    assert result["screen_changed"] is True


async def test_mouse_double_click_no_humanize(_mock_deps):
    mock_run, _, _, _, _ = _mock_deps
    result = await mouse_double_click(30, 40, button="right", humanize=False)
    mock_run.assert_any_await(["xdotool", "mousemove", "30", "40"])
    mock_run.assert_any_await(["xdotool", "click", "--repeat", "2", "--delay", "100", "3"])
    assert "Double-clicked right" in result["action"]


# --- mouse_drag ---

async def test_mouse_drag_humanize(_mock_deps):
    mock_run, mock_pos, mock_hmove, _, _ = _mock_deps
    result = await mouse_drag(10, 20, 100, 200, button="left", humanize=True)
    assert mock_hmove.await_count == 2
    assert mock_run.await_count == 2  # mousedown + mouseup
    mock_run.assert_any_await(["xdotool", "mousedown", "1"])
    mock_run.assert_any_await(["xdotool", "mouseup", "1"])
    assert result["action"] == "Dragged from (10, 20) to (100, 200)"
    assert result["screen_changed"] is True


async def test_mouse_drag_no_humanize(_mock_deps):
    mock_run, _, _, _, _ = _mock_deps
    result = await mouse_drag(10, 20, 100, 200, button="right", humanize=False)
    assert mock_run.await_count == 4  # 2x mousemove + mousedown + mouseup
    mock_run.assert_any_await(["xdotool", "mousemove", "10", "20"])
    mock_run.assert_any_await(["xdotool", "mousedown", "3"])
    mock_run.assert_any_await(["xdotool", "mousemove", "100", "200"])
    mock_run.assert_any_await(["xdotool", "mouseup", "3"])
    assert "Dragged from" in result["action"]


# --- mouse_scroll ---

async def test_mouse_scroll_down(_mock_deps):
    mock_run, _, mock_hmove, _, _ = _mock_deps
    result = await mouse_scroll(50, 50, direction="down", amount=3, humanize=True)
    mock_hmove.assert_awaited_once()
    mock_run.assert_awaited_once_with(["xdotool", "click", "--repeat", "3", "--delay", "50", "5"])
    assert result["action"] == "Scrolled down 3 clicks at (50, 50)"
    assert result["screen_changed"] is True


async def test_mouse_scroll_up(_mock_deps):
    mock_run, _, _, _, _ = _mock_deps
    await mouse_scroll(50, 50, direction="up", amount=5, humanize=False)
    mock_run.assert_any_await(["xdotool", "click", "--repeat", "5", "--delay", "50", "4"])


async def test_mouse_scroll_left(_mock_deps):
    mock_run, _, _, _, _ = _mock_deps
    await mouse_scroll(0, 0, direction="left", amount=1, humanize=False)
    mock_run.assert_any_await(["xdotool", "click", "--repeat", "1", "--delay", "50", "6"])


async def test_mouse_scroll_right(_mock_deps):
    mock_run, _, _, _, _ = _mock_deps
    await mouse_scroll(0, 0, direction="right", amount=2, humanize=False)
    mock_run.assert_any_await(["xdotool", "click", "--repeat", "2", "--delay", "50", "7"])


async def test_mouse_scroll_unknown_direction_defaults_to_down(_mock_deps):
    mock_run, _, _, _, _ = _mock_deps
    await mouse_scroll(0, 0, direction="diagonal", amount=1, humanize=False)
    mock_run.assert_any_await(["xdotool", "click", "--repeat", "1", "--delay", "50", "5"])


async def test_mouse_scroll_no_humanize(_mock_deps):
    mock_run, _, mock_hmove, _, _ = _mock_deps
    await mouse_scroll(50, 50, direction="down", amount=3, humanize=False)
    mock_hmove.assert_not_awaited()
    mock_run.assert_any_await(["xdotool", "mousemove", "50", "50"])
