# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.clipboard.get — read clipboard content via wl-paste."""

from unittest.mock import AsyncMock, patch

from ghostdesk.clipboard.get import get_clipboard


async def test_get_clipboard_success():
    """get_clipboard() returns text from wl-paste on success."""
    with patch("ghostdesk.clipboard.get.run", new_callable=AsyncMock) as mock_run:
        mock_run.return_value = "clipboard contents"

        result = await get_clipboard()

        assert result == "clipboard contents"
        mock_run.assert_awaited_once_with(["wl-paste", "--no-newline"])


async def test_get_clipboard_runtime_error_returns_empty():
    """get_clipboard() returns empty string when wl-paste raises RuntimeError."""
    with patch("ghostdesk.clipboard.get.run", new_callable=AsyncMock) as mock_run:
        mock_run.side_effect = RuntimeError("wl-paste failed")

        result = await get_clipboard()

        assert result == ""


async def test_get_clipboard_empty_clipboard():
    """get_clipboard() returns empty string when clipboard is empty."""
    with patch("ghostdesk.clipboard.get.run", new_callable=AsyncMock) as mock_run:
        mock_run.return_value = ""

        result = await get_clipboard()

        assert result == ""
