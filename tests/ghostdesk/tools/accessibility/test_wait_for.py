# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.accessibility.wait_for."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.accessibility.wait_for import wait_for_element

MODULE = "ghostdesk.tools.accessibility.wait_for"


@pytest.fixture(autouse=True)
def _mock_run_atspi():
    with patch(f"{MODULE}.run_atspi", new_callable=AsyncMock) as mock:
        yield mock


async def test_wait_found_immediately(_mock_run_atspi):
    _mock_run_atspi.return_value = {"name": "OK", "role": "button", "center_x": 50, "center_y": 60}

    result = await wait_for_element("OK", timeout_seconds=0.5, poll_interval_ms=50)

    assert result["status"] == "ready"
    assert result["element"]["center_x"] == 50
    _mock_run_atspi.assert_awaited_once()


async def test_wait_found_after_retries(_mock_run_atspi):
    no_match = {"name": "OK", "role": "button"}  # no center_x
    match = {"name": "OK", "role": "button", "center_x": 50, "center_y": 60}
    _mock_run_atspi.side_effect = [no_match, no_match, match]

    result = await wait_for_element("OK", timeout_seconds=2.0, poll_interval_ms=50)

    assert result["status"] == "ready"
    assert _mock_run_atspi.await_count == 3


async def test_wait_found_after_runtime_errors(_mock_run_atspi):
    """RuntimeError from run_atspi should be swallowed and retried."""
    match = {"name": "OK", "role": "button", "center_x": 50, "center_y": 60}
    _mock_run_atspi.side_effect = [RuntimeError("not ready"), RuntimeError("still not"), match]

    result = await wait_for_element("OK", timeout_seconds=2.0, poll_interval_ms=50)

    assert result["status"] == "ready"


async def test_wait_timeout(_mock_run_atspi):
    _mock_run_atspi.return_value = {"name": "OK", "role": "button"}  # no center_x ever

    result = await wait_for_element("OK", timeout_seconds=0.15, poll_interval_ms=50)

    assert result["status"] == "timeout"
    assert "not found" in result["error"]


async def test_wait_timeout_all_errors(_mock_run_atspi):
    _mock_run_atspi.side_effect = RuntimeError("always fails")

    result = await wait_for_element("Missing", timeout_seconds=0.15, poll_interval_ms=50)

    assert result["status"] == "timeout"
    assert "Missing" in result["error"]


async def test_wait_with_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"name": "OK", "role": "button", "center_x": 1, "center_y": 2}

    await wait_for_element("OK", role="button", timeout_seconds=0.5, poll_interval_ms=50)

    _mock_run_atspi.assert_awaited_once_with("find", ["OK", "--role", "button"])


async def test_wait_without_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"name": "OK", "role": "button", "center_x": 1, "center_y": 2}

    await wait_for_element("OK", timeout_seconds=0.5, poll_interval_ms=50)

    _mock_run_atspi.assert_awaited_once_with("find", ["OK"])
