# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk.input.mouse.

The mouse module is a thin orchestration layer over ``WaylandInput``.
We mock the singleton and assert on the ordered sequence of calls it
receives, which keeps the tests close to the real wire behaviour while
staying pure unit tests (no real Wayland connection needed).
"""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.input.mouse import (
    mouse_click,
    mouse_double_click,
    mouse_drag,
    mouse_scroll,
)

_FEEDBACK_RESULT = {"changed": True, "reaction_time_ms": 150}


@pytest.fixture
def _mock_deps():
    mock_wl = AsyncMock()

    with (
        patch("ghostdesk.input.mouse.get_wayland_input", new=AsyncMock(return_value=mock_wl)),
        patch("ghostdesk.input.mouse.capture_before", new_callable=AsyncMock, return_value=(None, b"h")) as mock_cap,
        patch("ghostdesk.input.mouse.poll_for_change", new_callable=AsyncMock, return_value=_FEEDBACK_RESULT) as mock_poll,
    ):
        yield mock_wl, mock_cap, mock_poll


# --- mouse_click ---

async def test_mouse_click_left(_mock_deps):
    """Left click moves the pointer then clicks left."""
    mock_wl, mock_cap, _ = _mock_deps
    result = await mouse_click(640, 512, button="left")

    mock_wl.move.assert_awaited_once_with(640, 512)
    mock_wl.click.assert_awaited_once_with("left")
    mock_cap.assert_awaited_once_with(640, 512)
    assert result["action"] == "Clicked left at (640, 512)"
    assert result["screen_changed"] is True


async def test_mouse_click_right(_mock_deps):
    mock_wl, *_ = _mock_deps
    await mouse_click(100, 200, button="right")
    mock_wl.click.assert_awaited_once_with("right")


async def test_mouse_click_middle(_mock_deps):
    mock_wl, *_ = _mock_deps
    await mouse_click(0, 0, button="middle")
    mock_wl.click.assert_awaited_once_with("middle")


async def test_mouse_click_passes_raw_pixels(_mock_deps):
    """mouse_click forwards screen pixels straight through to WaylandInput."""
    mock_wl, *_ = _mock_deps
    await mouse_click(-100, 5000, button="left")
    mock_wl.move.assert_awaited_once_with(-100, 5000)


async def test_mouse_click_no_change(_mock_deps):
    _, _, mock_poll = _mock_deps
    mock_poll.return_value = {"changed": False, "reaction_time_ms": 2000}
    result = await mouse_click(50, 60)
    assert result["screen_changed"] is False


# --- mouse_double_click ---

async def test_mouse_double_click(_mock_deps):
    mock_wl, *_ = _mock_deps
    result = await mouse_double_click(30, 40, button="left")

    mock_wl.move.assert_awaited_once_with(30, 40)
    assert mock_wl.click.await_count == 2
    mock_wl.click.assert_awaited_with("left")
    assert result["action"] == "Double-clicked left at (30, 40)"


async def test_mouse_double_click_right(_mock_deps):
    mock_wl, *_ = _mock_deps
    await mouse_double_click(30, 40, button="right")
    assert mock_wl.click.await_count == 2
    mock_wl.click.assert_awaited_with("right")


# --- mouse_drag ---

async def test_mouse_drag(_mock_deps):
    """Drag delegates to the batched WaylandInput.drag method."""
    mock_wl, mock_cap, _ = _mock_deps
    result = await mouse_drag(10, 20, 100, 200, button="left")

    mock_wl.drag.assert_awaited_once_with(10, 20, 100, 200, "left")
    mock_cap.assert_awaited_once_with(100, 200)
    assert result["action"] == "Dragged from (10, 20) to (100, 200)"


async def test_mouse_drag_right_button(_mock_deps):
    mock_wl, *_ = _mock_deps
    await mouse_drag(10, 20, 100, 200, button="right")
    mock_wl.drag.assert_awaited_once_with(10, 20, 100, 200, "right")


# --- mouse_scroll ---

async def test_mouse_scroll_down(_mock_deps):
    mock_wl, *_ = _mock_deps
    result = await mouse_scroll(50, 50, direction="down", amount=3)
    mock_wl.move.assert_awaited_once_with(50, 50)
    mock_wl.scroll.assert_awaited_once_with("down", 3)
    assert result["action"] == "Scrolled down 3 clicks at (50, 50)"


async def test_mouse_scroll_up(_mock_deps):
    mock_wl, *_ = _mock_deps
    await mouse_scroll(50, 50, direction="up", amount=2)
    mock_wl.scroll.assert_awaited_once_with("up", 2)


async def test_mouse_scroll_left(_mock_deps):
    mock_wl, *_ = _mock_deps
    await mouse_scroll(0, 0, direction="left", amount=1)
    mock_wl.scroll.assert_awaited_once_with("left", 1)


async def test_mouse_scroll_right(_mock_deps):
    mock_wl, *_ = _mock_deps
    await mouse_scroll(0, 0, direction="right", amount=2)
    mock_wl.scroll.assert_awaited_once_with("right", 2)


async def test_mouse_scroll_amount_clamped(_mock_deps):
    """Amount is clamped to [1, 5]."""
    mock_wl, *_ = _mock_deps
    await mouse_scroll(0, 0, direction="down", amount=10)
    mock_wl.scroll.assert_awaited_with("down", 5)

    mock_wl.scroll.reset_mock()
    await mouse_scroll(0, 0, direction="down", amount=0)
    mock_wl.scroll.assert_awaited_with("down", 1)


# --- ctx-based MCP logging ---

async def test_mouse_click_warns_client_on_miss(_mock_deps):
    """A miss pushes a warning through ctx."""
    _, _, mock_poll = _mock_deps
    mock_poll.return_value = {"changed": False, "reaction_time_ms": 2000}

    ctx = AsyncMock()
    await mouse_click(50, 60, ctx=ctx)

    ctx.warning.assert_awaited_once()
    assert "no visible screen change" in ctx.warning.await_args.args[0]


async def test_mouse_click_does_not_warn_on_success(_mock_deps):
    """A successful click pushes no warning."""
    ctx = AsyncMock()
    await mouse_click(50, 60, ctx=ctx)
    ctx.warning.assert_not_awaited()
