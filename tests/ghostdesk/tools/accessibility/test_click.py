# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.accessibility.click."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.accessibility.click import click_element

MODULE = "ghostdesk.tools.accessibility.click"


@pytest.fixture(autouse=True)
def _mock_deps():
    with (
        patch(f"{MODULE}.run_atspi", new_callable=AsyncMock) as mock_atspi,
        patch(f"{MODULE}.run", new_callable=AsyncMock) as mock_run,
        patch(f"{MODULE}.get_cursor_position", new_callable=AsyncMock, return_value=(10, 20)) as mock_pos,
        patch(f"{MODULE}.human_move", new_callable=AsyncMock) as mock_hmove,
    ):
        yield mock_atspi, mock_run, mock_pos, mock_hmove


async def test_click_element_found_humanize(_mock_deps):
    mock_atspi, mock_run, mock_pos, mock_hmove = _mock_deps
    mock_atspi.return_value = {"name": "Submit", "role": "button", "center_x": 100, "center_y": 200}

    result = await click_element("Submit", humanize=True)

    mock_atspi.assert_awaited_once_with("find", ["Submit"])
    mock_pos.assert_awaited_once()
    mock_hmove.assert_awaited_once_with(10, 20, 100, 200)
    mock_run.assert_awaited_once_with(["xdotool", "click", "1"])
    assert "Submit" in result
    assert "button" in result
    assert "(100, 200)" in result


async def test_click_element_found_no_humanize(_mock_deps):
    mock_atspi, mock_run, _, mock_hmove = _mock_deps
    mock_atspi.return_value = {"name": "Cancel", "role": "button", "center_x": 50, "center_y": 60}

    result = await click_element("Cancel", humanize=False)

    mock_hmove.assert_not_awaited()
    # Two calls: mousemove + click
    assert mock_run.await_count == 2
    mock_run.assert_any_await(["xdotool", "mousemove", "50", "60"])
    mock_run.assert_any_await(["xdotool", "click", "1"])
    assert "Cancel" in result


async def test_click_element_not_found(_mock_deps):
    mock_atspi, mock_run, _, _ = _mock_deps
    mock_atspi.return_value = {"error_detail": "nothing here"}  # no center_x key

    result = await click_element("Nonexistent")

    assert "No element found" in result
    mock_run.assert_not_awaited()


async def test_click_element_with_role(_mock_deps):
    mock_atspi, _, _, _ = _mock_deps
    mock_atspi.return_value = {"name": "OK", "role": "button", "center_x": 1, "center_y": 2}

    await click_element("OK", role="button")

    mock_atspi.assert_awaited_once_with("find", ["OK", "--role", "button"])


async def test_click_element_no_role(_mock_deps):
    mock_atspi, _, _, _ = _mock_deps
    mock_atspi.return_value = {"name": "OK", "role": "button", "center_x": 1, "center_y": 2}

    await click_element("OK")

    mock_atspi.assert_awaited_once_with("find", ["OK"])
