# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.accessibility.read."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.accessibility.read import (
    get_element_details,
    read_screen,
    read_table,
)

MODULE = "ghostdesk.tools.accessibility.read"


@pytest.fixture(autouse=True)
def _mock_run_atspi():
    with patch(f"{MODULE}.run_atspi", new_callable=AsyncMock) as mock:
        yield mock


# --- read_screen ---

async def test_read_screen_default(_mock_run_atspi):
    _mock_run_atspi.return_value = {
        "items": [{"role": "heading", "name": "Welcome"}],
        "visible": 1,
        "total_in_tree": 10,
    }
    result = await read_screen()
    _mock_run_atspi.assert_awaited_once_with("read", ["--max", "500"])
    assert result["items"][0]["name"] == "Welcome"
    assert result["visible"] == 1
    assert result["total_in_tree"] == 10


async def test_read_screen_custom_max(_mock_run_atspi):
    _mock_run_atspi.return_value = {"items": [], "visible": 0, "total_in_tree": 0}
    await read_screen(max_results=10)
    _mock_run_atspi.assert_awaited_once_with("read", ["--max", "10"])


async def test_read_screen_with_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"items": [], "visible": 0, "total_in_tree": 0}
    await read_screen(role="button", max_results=50)
    _mock_run_atspi.assert_awaited_once_with("read", ["--max", "50", "--role", "button"])


async def test_read_screen_invalid_role(_mock_run_atspi):
    with pytest.raises(ValueError, match="Invalid role 'bogus'"):
        await read_screen(role="bogus")
    _mock_run_atspi.assert_not_awaited()


async def test_read_screen_all_valid_roles(_mock_run_atspi):
    """Every role in VALID_ROLES should pass validation."""
    from ghostdesk.tools.accessibility._client import VALID_ROLES

    _mock_run_atspi.return_value = {"items": [], "visible": 0, "total_in_tree": 0}
    for role in VALID_ROLES:
        await read_screen(role=role)  # should not raise


# --- get_element_details ---

async def test_get_element_details_no_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"name": "Submit", "role": "button", "states": ["enabled"]}
    result = await get_element_details("Submit")
    _mock_run_atspi.assert_awaited_once_with("details", ["Submit"])
    assert result["name"] == "Submit"


async def test_get_element_details_with_role(_mock_run_atspi):
    _mock_run_atspi.return_value = {"name": "Submit", "role": "button"}
    result = await get_element_details("Submit", role="button")
    _mock_run_atspi.assert_awaited_once_with("details", ["Submit", "--role", "button"])


# --- read_table ---

async def test_read_table_no_text(_mock_run_atspi):
    _mock_run_atspi.return_value = {"headers": ["A", "B"], "rows": [["1", "2"]]}
    result = await read_table()
    _mock_run_atspi.assert_awaited_once_with("table", ["--max-rows", "100"])
    assert result["headers"] == ["A", "B"]


async def test_read_table_with_text(_mock_run_atspi):
    _mock_run_atspi.return_value = {"headers": [], "rows": []}
    await read_table(text="Sales", max_rows=50)
    _mock_run_atspi.assert_awaited_once_with("table", ["--max-rows", "50", "--text", "Sales"])


async def test_read_table_no_text_custom_max(_mock_run_atspi):
    _mock_run_atspi.return_value = {}
    await read_table(max_rows=10)
    _mock_run_atspi.assert_awaited_once_with("table", ["--max-rows", "10"])
