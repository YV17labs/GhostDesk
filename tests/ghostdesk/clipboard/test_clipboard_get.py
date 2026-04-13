# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.clipboard.clipboard_get — read clipboard content via wl-paste."""

from unittest.mock import AsyncMock, patch

from ghostdesk.clipboard.clipboard_get import clipboard_get


async def test_clipboard_get_success():
    """clipboard_get() returns text from wl-paste on success."""
    with patch("ghostdesk.clipboard.clipboard_get.run", new_callable=AsyncMock) as mock_run:
        mock_run.return_value = "clipboard contents"

        result = await clipboard_get()

        assert result == "clipboard contents"
        mock_run.assert_awaited_once_with(["wl-paste", "--no-newline"])


async def test_clipboard_get_runtime_error_returns_empty():
    """clipboard_get() returns empty string when wl-paste raises RuntimeError."""
    with patch("ghostdesk.clipboard.clipboard_get.run", new_callable=AsyncMock) as mock_run:
        mock_run.side_effect = RuntimeError("wl-paste failed")

        result = await clipboard_get()

        assert result == ""


async def test_clipboard_get_empty_clipboard():
    """clipboard_get() returns empty string when clipboard is empty."""
    with patch("ghostdesk.clipboard.clipboard_get.run", new_callable=AsyncMock) as mock_run:
        mock_run.return_value = ""

        result = await clipboard_get()

        assert result == ""
