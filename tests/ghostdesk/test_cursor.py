# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk._cursor — cursor position utility."""

from unittest.mock import AsyncMock, patch

from ghostdesk._cursor import get_cursor_position

MODULE = "ghostdesk._cursor"


async def test_parses_xdotool_output():
    """get_cursor_position should parse xdotool getmouselocation output."""
    with patch(f"{MODULE}.run", new_callable=AsyncMock) as mock_run:
        mock_run.return_value = "x:123 y:456 screen:0 window:789"
        x, y = await get_cursor_position()
        assert x == 123
        assert y == 456
        mock_run.assert_awaited_once_with(["xdotool", "getmouselocation"])


async def test_parses_different_coordinates():
    """get_cursor_position works with various coordinate values."""
    with patch(f"{MODULE}.run", new_callable=AsyncMock) as mock_run:
        mock_run.return_value = "x:0 y:0 screen:0 window:12345"
        x, y = await get_cursor_position()
        assert x == 0
        assert y == 0
