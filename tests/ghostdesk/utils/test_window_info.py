# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.utils.window_info — window information utilities."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.utils.window_info import _parse_geometry, get_window_info


# ---------------------------------------------------------------------------
# Pure function tests
# ---------------------------------------------------------------------------

class TestParseGeometry:
    """Tests for _parse_geometry."""

    def test_standard_xdotool_output(self):
        """Parse typical xdotool getwindowgeometry output."""
        raw = (
            "Window 12345678\n"
            "  Position: 10,20 (screen: 0)\n"
            "  Geometry: 800x600\n"
        )
        result = _parse_geometry(raw)
        assert result == {"x": 10, "y": 20, "width": 800, "height": 600}

    def test_zero_position(self):
        """Parse geometry with zero position."""
        raw = (
            "Window 99999\n"
            "  Position: 0,0 (screen: 0)\n"
            "  Geometry: 1920x1080\n"
        )
        result = _parse_geometry(raw)
        assert result == {"x": 0, "y": 0, "width": 1920, "height": 1080}

    def test_large_coordinates(self):
        """Parse geometry with large coordinate values."""
        raw = (
            "Window 1\n"
            "  Position: 3840,2160 (screen: 1)\n"
            "  Geometry: 2560x1440\n"
        )
        result = _parse_geometry(raw)
        assert result == {"x": 3840, "y": 2160, "width": 2560, "height": 1440}

    def test_missing_position_line(self):
        """When Position line is missing, x/y default to 0."""
        raw = "  Geometry: 640x480\n"
        result = _parse_geometry(raw)
        assert result["x"] == 0
        assert result["y"] == 0
        assert result["width"] == 640
        assert result["height"] == 480

    def test_missing_geometry_line(self):
        """When Geometry line is missing, width/height default to 0."""
        raw = "  Position: 100,200 (screen: 0)\n"
        result = _parse_geometry(raw)
        assert result["x"] == 100
        assert result["y"] == 200
        assert result["width"] == 0
        assert result["height"] == 0

    def test_empty_input(self):
        """Empty string returns all zeros."""
        result = _parse_geometry("")
        assert result == {"x": 0, "y": 0, "width": 0, "height": 0}

    def test_unrelated_lines_ignored(self):
        """Lines that don't match Position/Geometry are ignored."""
        raw = (
            "Window 12345\n"
            "  SomeOther: value\n"
            "  Position: 50,75 (screen: 0)\n"
            "  Extra: stuff\n"
            "  Geometry: 300x200\n"
        )
        result = _parse_geometry(raw)
        assert result == {"x": 50, "y": 75, "width": 300, "height": 200}


# ---------------------------------------------------------------------------
# Async function tests
# ---------------------------------------------------------------------------

class TestGetWindowInfo:
    """Tests for get_window_info."""

    async def test_success_full_info(self):
        """get_window_info returns active window and window list on success."""
        async def mock_run_side_effect(cmd, **kwargs):
            if cmd == ["xdotool", "getactivewindow"]:
                return "12345"
            elif cmd == ["xdotool", "getwindowname", "12345"]:
                return "My Editor"
            elif cmd == ["xdotool", "search", "--onlyvisible", "--name", ""]:
                return "12345\n67890"
            elif cmd == ["xdotool", "getwindowname", "67890"]:
                return "Terminal"
            elif cmd == ["xdotool", "getwindowgeometry", "12345"]:
                return "Window 12345\n  Position: 0,0 (screen: 0)\n  Geometry: 800x600"
            elif cmd == ["xdotool", "getwindowgeometry", "67890"]:
                return "Window 67890\n  Position: 810,0 (screen: 0)\n  Geometry: 600x400"
            return ""

        with patch("ghostdesk.utils.window_info.run", new_callable=AsyncMock) as mock_run:
            mock_run.side_effect = mock_run_side_effect

            info = await get_window_info()

            assert info["active_window"] == "My Editor"
            assert len(info["windows"]) == 2
            assert info["windows"][0]["title"] == "My Editor"
            assert info["windows"][0]["width"] == 800
            assert info["windows"][1]["title"] == "Terminal"

    async def test_active_window_failure_returns_none(self):
        """When getactivewindow fails, active_window should be None."""
        async def mock_run_side_effect(cmd, **kwargs):
            if cmd == ["xdotool", "getactivewindow"]:
                raise RuntimeError("No active window")
            elif cmd == ["xdotool", "search", "--onlyvisible", "--name", ""]:
                return ""
            return ""

        with patch("ghostdesk.utils.window_info.run", new_callable=AsyncMock) as mock_run:
            mock_run.side_effect = mock_run_side_effect

            info = await get_window_info()

            assert info["active_window"] is None

    async def test_window_search_failure_returns_empty_list(self):
        """When window search fails, windows list should be empty."""
        async def mock_run_side_effect(cmd, **kwargs):
            if cmd == ["xdotool", "getactivewindow"]:
                return "12345"
            elif cmd == ["xdotool", "getwindowname", "12345"]:
                return "Editor"
            elif cmd == ["xdotool", "search", "--onlyvisible", "--name", ""]:
                raise RuntimeError("Search failed")
            return ""

        with patch("ghostdesk.utils.window_info.run", new_callable=AsyncMock) as mock_run:
            mock_run.side_effect = mock_run_side_effect

            info = await get_window_info()

            assert info["active_window"] == "Editor"
            assert info["windows"] == []

    async def test_individual_window_failure_skipped(self):
        """Windows that fail to fetch name/geometry are excluded."""
        async def mock_run_side_effect(cmd, **kwargs):
            if cmd == ["xdotool", "getactivewindow"]:
                return "111"
            elif cmd == ["xdotool", "getwindowname", "111"]:
                return "Good Window"
            elif cmd == ["xdotool", "search", "--onlyvisible", "--name", ""]:
                return "111\n222"
            elif cmd == ["xdotool", "getwindowname", "222"]:
                raise RuntimeError("Window gone")
            elif cmd == ["xdotool", "getwindowgeometry", "111"]:
                return "Window 111\n  Position: 0,0 (screen: 0)\n  Geometry: 400x300"
            return ""

        with patch("ghostdesk.utils.window_info.run", new_callable=AsyncMock) as mock_run:
            mock_run.side_effect = mock_run_side_effect

            info = await get_window_info()

            # Only window 111 should be in the list; 222 failed
            assert len(info["windows"]) == 1
            assert info["windows"][0]["title"] == "Good Window"

    async def test_window_with_empty_name_skipped(self):
        """Windows with empty names are filtered out."""
        async def mock_run_side_effect(cmd, **kwargs):
            if cmd == ["xdotool", "getactivewindow"]:
                return "111"
            elif cmd == ["xdotool", "getwindowname", "111"]:
                return ""
            elif cmd == ["xdotool", "search", "--onlyvisible", "--name", ""]:
                return "111"
            elif cmd == ["xdotool", "getwindowgeometry", "111"]:
                return "Window 111\n  Position: 0,0 (screen: 0)\n  Geometry: 400x300"
            return ""

        with patch("ghostdesk.utils.window_info.run", new_callable=AsyncMock) as mock_run:
            mock_run.side_effect = mock_run_side_effect

            info = await get_window_info()

            # Active window should be None (empty name)
            assert info["active_window"] is None
            # Window with empty name should be excluded from list
            assert info["windows"] == []

    async def test_all_failures_returns_default_structure(self):
        """When everything fails, return the default dict structure."""
        with patch("ghostdesk.utils.window_info.run", new_callable=AsyncMock) as mock_run:
            mock_run.side_effect = RuntimeError("Everything broken")

            info = await get_window_info()

            assert info == {"active_window": None, "windows": []}
