# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.accessibility.interact."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.accessibility.interact import (
    focus_element,
    scroll_to_element,
    set_value,
)

MODULE = "ghostdesk.tools.accessibility.interact"


@pytest.fixture(autouse=True)
def _mock_run_atspi():
    with patch(f"{MODULE}.run_atspi", new_callable=AsyncMock) as mock:
        yield mock


# --- set_value ---

async def test_set_value_no_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"status": "ok"}
    result = await set_value("Username", "alice")
    _mock_run_atspi.assert_awaited_once_with("set-value", ["Username", "alice"])
    assert result == {"status": "ok"}


async def test_set_value_with_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"status": "ok"}
    result = await set_value("Volume", "50", role="slider")
    _mock_run_atspi.assert_awaited_once_with("set-value", ["Volume", "50", "--role", "slider"])


# --- focus_element ---

async def test_focus_element_no_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"status": "focused"}
    result = await focus_element("Search")
    _mock_run_atspi.assert_awaited_once_with("focus", ["Search"])
    assert result["status"] == "focused"


async def test_focus_element_with_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"status": "focused"}
    await focus_element("Search", role="textfield")
    _mock_run_atspi.assert_awaited_once_with("focus", ["Search", "--role", "textfield"])


# --- scroll_to_element ---

async def test_scroll_to_element_no_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"status": "scrolled"}
    result = await scroll_to_element("Footer")
    _mock_run_atspi.assert_awaited_once_with("scroll", ["Footer"])
    assert result["status"] == "scrolled"


async def test_scroll_to_element_with_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"status": "scrolled"}
    await scroll_to_element("Footer", role="link")
    _mock_run_atspi.assert_awaited_once_with("scroll", ["Footer", "--role", "link"])


# --- error handling ---

async def test_set_value_error_dict_gets_hint(_mock_run_atspi):
    """When AT-SPI returns an error dict (exit 0), hint is added."""
    _mock_run_atspi.return_value = {"error": "Element not found: 'Missing'"}
    result = await set_value("Missing", "val")
    assert result["error"] == "Element not found: 'Missing'"
    assert "hint" in result


async def test_focus_element_runtime_error_gets_hint(_mock_run_atspi):
    """When run_atspi raises RuntimeError, hint is included."""
    _mock_run_atspi.side_effect = RuntimeError("AT-SPI query failed: No display")
    result = await focus_element("Search")
    assert "error" in result
    assert "hint" in result
