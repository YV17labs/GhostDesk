# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.accessibility.read."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.accessibility.read import (
    get_element_details,
    list_elements,
    read_screen,
    read_table,
)

MODULE = "ghostdesk.tools.accessibility.read"


@pytest.fixture(autouse=True)
def _mock_run_atspi():
    with patch(f"{MODULE}.run_atspi", new_callable=AsyncMock) as mock:
        yield mock


# --- list_elements ---

async def test_list_elements_no_role(_mock_run_atspi):
    _mock_run_atspi.return_value = [{"role": "button", "name": "OK"}]
    result = await list_elements()
    _mock_run_atspi.assert_awaited_once_with("elements", ["--max", "100"])
    assert len(result) == 1


async def test_list_elements_with_role(_mock_run_atspi):
    _mock_run_atspi.return_value = []
    result = await list_elements(role="button", max_results=50)
    _mock_run_atspi.assert_awaited_once_with("elements", ["--max", "50", "--role", "button"])
    assert result == []


async def test_list_elements_invalid_role(_mock_run_atspi):
    with pytest.raises(ValueError, match="Invalid role 'bogus'"):
        await list_elements(role="bogus")
    _mock_run_atspi.assert_not_awaited()


async def test_list_elements_all_valid_roles(_mock_run_atspi):
    """Every role in VALID_ROLES should pass validation."""
    from ghostdesk.tools.accessibility._client import VALID_ROLES

    _mock_run_atspi.return_value = []
    for role in VALID_ROLES:
        await list_elements(role=role)  # should not raise


# --- read_screen ---

async def test_read_screen_default(_mock_run_atspi):
    _mock_run_atspi.return_value = [{"role": "heading", "text": "Welcome"}]
    result = await read_screen()
    _mock_run_atspi.assert_awaited_once_with("text", ["--max", "500"])
    assert result[0]["text"] == "Welcome"


async def test_read_screen_custom_max(_mock_run_atspi):
    _mock_run_atspi.return_value = []
    await read_screen(max_results=10)
    _mock_run_atspi.assert_awaited_once_with("text", ["--max", "10"])


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
