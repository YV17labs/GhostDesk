# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.tools.clipboard.get — read clipboard content."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.clipboard.get import get_clipboard


async def test_get_clipboard_success():
    """get_clipboard() returns text from xclip on success."""
    with patch("ghostdesk.tools.clipboard.get.run", new_callable=AsyncMock) as mock_run:
        mock_run.return_value = "clipboard contents"

        result = await get_clipboard()

        assert result == "clipboard contents"
        mock_run.assert_awaited_once_with(
            ["xclip", "-selection", "clipboard", "-o"],
        )


async def test_get_clipboard_runtime_error_returns_empty():
    """get_clipboard() returns empty string when xclip raises RuntimeError."""
    with patch("ghostdesk.tools.clipboard.get.run", new_callable=AsyncMock) as mock_run:
        mock_run.side_effect = RuntimeError("xclip failed")

        result = await get_clipboard()

        assert result == ""


async def test_get_clipboard_empty_clipboard():
    """get_clipboard() returns empty string when clipboard is empty."""
    with patch("ghostdesk.tools.clipboard.get.run", new_callable=AsyncMock) as mock_run:
        mock_run.return_value = ""

        result = await get_clipboard()

        assert result == ""
