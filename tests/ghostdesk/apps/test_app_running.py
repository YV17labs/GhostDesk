# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk.apps.app_running."""

import json
from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.apps.app_running import app_running

MODULE = "ghostdesk.apps.app_running"


@pytest.fixture
def mock_swaymsg():
    with patch("ghostdesk._sway._swaymsg", new_callable=AsyncMock) as mock:
        yield mock


def _tree_with(*views: dict) -> dict:
    return {
        "id": 1, "type": "root",
        "nodes": [{
            "id": 2, "type": "output",
            "nodes": [{
                "id": 3, "type": "workspace",
                "nodes": list(views),
                "floating_nodes": [],
            }],
            "floating_nodes": [],
        }],
        "floating_nodes": [],
    }


async def test_app_running_returns_open_views(mock_swaymsg):
    tree = _tree_with(
        {
            "id": 10, "type": "con", "pid": 1234,
            "app_id": "firefox", "name": "Mozilla — Reddit",
            "focused": True, "nodes": [], "floating_nodes": [],
        },
        {
            "id": 11, "type": "con", "pid": 5678,
            "app_id": "foot", "name": "agent@host:~",
            "focused": False, "nodes": [], "floating_nodes": [],
        },
    )
    mock_swaymsg.return_value = json.dumps(tree)

    result = await app_running()

    assert result == [
        {"app": "firefox", "title": "Mozilla — Reddit", "pid": 1234, "focused": True},
        {"app": "foot", "title": "agent@host:~", "pid": 5678, "focused": False},
    ]


async def test_app_running_empty_when_no_views(mock_swaymsg):
    """Bare desktop (no app open) returns an empty list, not an error."""
    mock_swaymsg.return_value = json.dumps(_tree_with())
    assert await app_running() == []


async def test_app_running_skips_non_view_nodes(mock_swaymsg):
    """Workspaces and outputs have no pid; they must not appear in the listing."""
    tree = _tree_with(
        {
            "id": 10, "type": "con", "pid": 99,
            "app_id": "mousepad", "name": "Untitled",
            "nodes": [], "floating_nodes": [],
        },
    )
    mock_swaymsg.return_value = json.dumps(tree)

    result = await app_running()

    assert len(result) == 1
    assert result[0]["app"] == "mousepad"


async def test_app_running_walks_floating_nodes(mock_swaymsg):
    tree = {
        "id": 1, "type": "root",
        "nodes": [{
            "id": 2, "type": "output",
            "nodes": [{
                "id": 3, "type": "workspace",
                "nodes": [],
                "floating_nodes": [
                    {
                        "id": 99, "type": "con", "pid": 7,
                        "app_id": "galculator", "name": "Calculator",
                        "nodes": [], "floating_nodes": [],
                    },
                ],
            }],
            "floating_nodes": [],
        }],
        "floating_nodes": [],
    }
    mock_swaymsg.return_value = json.dumps(tree)

    result = await app_running()

    assert len(result) == 1
    assert result[0]["app"] == "galculator"


async def test_app_running_handles_missing_app_id(mock_swaymsg):
    """A view without app_id but with window_properties.class falls back to class."""
    tree = _tree_with(
        {
            "id": 10, "type": "con", "pid": 1,
            "name": "Some X11-ish window",
            "window_properties": {"class": "LegacyApp"},
            "nodes": [], "floating_nodes": [],
        },
    )
    mock_swaymsg.return_value = json.dumps(tree)

    result = await app_running()
    assert result[0]["app"] == "LegacyApp"


async def test_app_running_handles_get_tree_failure(mock_swaymsg):
    mock_swaymsg.side_effect = RuntimeError("swaymsg gone")
    result = await app_running()
    assert len(result) == 1
    assert "error" in result[0]


async def test_app_running_handles_malformed_json(mock_swaymsg):
    mock_swaymsg.return_value = "not json {"
    result = await app_running()
    assert len(result) == 1
    assert "error" in result[0]
