# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.accessibility.read."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.accessibility.read import read_screen

MODULE = "ghostdesk.tools.accessibility.read"

_EMPTY = {"items": [], "visible": 0, "total_in_tree": 0}


@pytest.fixture(autouse=True)
def _mock_run_atspi():
    with (
        patch(f"{MODULE}.run_atspi", new_callable=AsyncMock) as mock,
        patch(f"{MODULE}.get_active_window", new_callable=AsyncMock, return_value="Test Window"),
    ):
        yield mock


# --- read_screen ---

async def test_read_screen_default(_mock_run_atspi):
    """Positions are included by default."""
    _mock_run_atspi.return_value = {
        "items": [{"role": "heading", "name": "Welcome"}],
        "visible": 1,
        "total_in_tree": 10,
    }
    result = await read_screen()
    _mock_run_atspi.assert_awaited_once_with(
        "read", ["--max", "100", "--include-positions"],
    )
    assert result["items"][0]["name"] == "Welcome"
    assert result["active_window"] == "Test Window"


async def test_read_screen_no_positions(_mock_run_atspi):
    """Positions can be explicitly disabled."""
    _mock_run_atspi.return_value = {**_EMPTY}
    await read_screen(include_positions=False)
    _mock_run_atspi.assert_awaited_once_with(
        "read", ["--max", "100"],
    )


async def test_read_screen_custom_max(_mock_run_atspi):
    _mock_run_atspi.return_value = {**_EMPTY}
    await read_screen(max_results=10)
    _mock_run_atspi.assert_awaited_once_with(
        "read", ["--max", "10", "--include-positions"],
    )


async def test_read_screen_with_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {**_EMPTY}
    await read_screen(role="button", max_results=50)
    _mock_run_atspi.assert_awaited_once_with(
        "read", ["--max", "50", "--include-positions", "--role", "button"],
    )


async def test_read_screen_with_app(_mock_run_atspi):
    _mock_run_atspi.return_value = {**_EMPTY}
    await read_screen(app="Firefox")
    _mock_run_atspi.assert_awaited_once_with(
        "read", ["--max", "100", "--include-positions", "--app", "Firefox"],
    )


async def test_read_screen_role_alias(_mock_run_atspi):
    """Alias 'row' should resolve to 'table_row'."""
    _mock_run_atspi.return_value = {**_EMPTY}
    await read_screen(role="row")
    _mock_run_atspi.assert_awaited_once_with(
        "read", ["--max", "100", "--include-positions", "--role", "table_row"],
    )


async def test_read_screen_invalid_role(_mock_run_atspi):
    with pytest.raises(ValueError, match="Invalid role 'bogus'"):
        await read_screen(role="bogus")
    _mock_run_atspi.assert_not_awaited()


async def test_read_screen_all_valid_roles(_mock_run_atspi):
    """Every role in VALID_ROLES should pass validation."""
    from ghostdesk.tools.accessibility._client import VALID_ROLES

    _mock_run_atspi.return_value = {**_EMPTY}
    for role in VALID_ROLES:
        await read_screen(role=role)  # should not raise


async def test_read_screen_browser_split(_mock_run_atspi):
    """Browser chrome is passed through when present."""
    _mock_run_atspi.return_value = {
        "items": [{"role": "button", "name": "Submit", "y": 200, "x": 100}],
        "browser": [{"role": "button", "name": "Reload", "y": 44, "x": 78}],
        "visible": 2,
        "total_in_tree": 2,
    }
    result = await read_screen()
    assert result["items"][0]["name"] == "Submit"
    assert result["browser"][0]["name"] == "Reload"


async def test_read_screen_no_browser(_mock_run_atspi):
    """Non-browser apps return items without browser key."""
    _mock_run_atspi.return_value = {
        "items": [{"role": "button", "name": "OK"}],
        "visible": 1,
        "total_in_tree": 1,
    }
    result = await read_screen()
    assert "browser" not in result
    assert result["items"][0]["name"] == "OK"
