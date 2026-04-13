# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.screen.windows — parse swaymsg get_tree output."""

import json
from unittest.mock import AsyncMock, patch

from ghostdesk.screen.windows import _extract_windows, get_open_windows

MODULE = "ghostdesk.screen.windows"


def _leaf(app_id: str, name: str, pid: int, rect: dict) -> dict:
    return {
        "type": "con",
        "pid": pid,
        "app_id": app_id,
        "name": name,
        "rect": rect,
        "nodes": [],
        "floating_nodes": [],
    }


def _tree(leaves: list[dict]) -> dict:
    """Wrap leaves in a minimal sway tree root."""
    return {
        "type": "root",
        "nodes": [
            {
                "type": "output",
                "name": "HEADLESS-1",
                "nodes": [
                    {
                        "type": "workspace",
                        "name": "1",
                        "nodes": leaves,
                        "floating_nodes": [],
                    }
                ],
                "floating_nodes": [],
            }
        ],
        "floating_nodes": [],
    }


def test_extract_windows_single_leaf():
    tree = _tree([
        _leaf("org.gnome.TextEditor", "untitled.txt", 1234,
              {"x": 0, "y": 0, "width": 1280, "height": 1024}),
    ])
    windows = _extract_windows(tree)
    assert len(windows) == 1
    w = windows[0]
    assert w["app"] == "org.gnome.texteditor"
    assert w["title"] == "untitled.txt"
    assert w["x"] == 0 and w["y"] == 0
    assert w["width"] == 1280 and w["height"] == 1024


def test_extract_windows_multiple_leaves():
    tree = _tree([
        _leaf("firefox", "Mozilla", 100, {"x": 0, "y": 0, "width": 640, "height": 480}),
        _leaf("foot", "term", 200, {"x": 640, "y": 0, "width": 640, "height": 480}),
    ])
    windows = _extract_windows(tree)
    assert len(windows) == 2
    assert {w["app"] for w in windows} == {"firefox", "foot"}


def test_extract_windows_skips_untitled():
    """Leaves with an empty name are treated as not-yet-mapped and skipped."""
    tree = _tree([
        _leaf("foot", "", 100, {"x": 0, "y": 0, "width": 10, "height": 10}),
        _leaf("firefox", "Mozilla", 200, {"x": 0, "y": 0, "width": 640, "height": 480}),
    ])
    windows = _extract_windows(tree)
    assert len(windows) == 1
    assert windows[0]["app"] == "firefox"


def test_extract_windows_xwayland_fallback():
    """XWayland clients have no app_id — we fall back to window_properties.class."""
    leaf = {
        "type": "con",
        "pid": 500,
        "app_id": None,
        "name": "Some X Window",
        "rect": {"x": 0, "y": 0, "width": 200, "height": 100},
        "window_properties": {"class": "XTerm"},
        "nodes": [],
        "floating_nodes": [],
    }
    windows = _extract_windows(_tree([leaf]))
    assert len(windows) == 1
    assert windows[0]["app"] == "xterm"


def test_extract_windows_skips_non_leaf_containers():
    """Split containers (with nodes children) are not windows."""
    split = {
        "type": "con",
        "pid": 999,
        "app_id": "split",
        "name": "split",
        "rect": {"x": 0, "y": 0, "width": 10, "height": 10},
        "nodes": [
            _leaf("firefox", "Mozilla", 100, {"x": 0, "y": 0, "width": 640, "height": 480}),
        ],
        "floating_nodes": [],
    }
    windows = _extract_windows(_tree([split]))
    # Only the leaf firefox should come through, not the split parent
    assert len(windows) == 1
    assert windows[0]["app"] == "firefox"


async def test_get_open_windows_happy_path():
    tree = _tree([
        _leaf("firefox", "Mozilla", 100, {"x": 0, "y": 0, "width": 640, "height": 480}),
    ])
    with patch(f"{MODULE}.run", new_callable=AsyncMock) as mock_run:
        mock_run.return_value = json.dumps(tree)
        windows = await get_open_windows()
        assert len(windows) == 1
        mock_run.assert_awaited_once_with(["swaymsg", "-t", "get_tree"])


async def test_get_open_windows_returns_empty_on_failure():
    with patch(f"{MODULE}.run", new_callable=AsyncMock) as mock_run:
        mock_run.side_effect = RuntimeError("swaymsg not running")
        assert await get_open_windows() == []


async def test_get_open_windows_returns_empty_on_invalid_json():
    with patch(f"{MODULE}.run", new_callable=AsyncMock) as mock_run:
        mock_run.return_value = "not valid json {"
        assert await get_open_windows() == []
