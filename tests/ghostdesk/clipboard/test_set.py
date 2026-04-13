# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.clipboard.set_ — write text via wl-copy."""

import asyncio
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from ghostdesk.clipboard.set_ import set_clipboard


@pytest.fixture
def mock_process():
    """Create a mock asyncio.subprocess.Process with stdin mock."""
    proc = AsyncMock()
    proc.returncode = 0
    # stdin needs synchronous write() and close(), async wait() on the process
    proc.stdin = MagicMock()
    proc.stdin.write = MagicMock()
    proc.stdin.close = MagicMock()
    proc.wait = AsyncMock(return_value=0)
    return proc


@pytest.fixture
def patch_subprocess(mock_process):
    """Patch asyncio.create_subprocess_exec to return mock_process."""
    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock) as mock_exec:
        mock_exec.return_value = mock_process
        yield mock_exec, mock_process


async def test_set_clipboard_success(patch_subprocess):
    """set_clipboard() returns confirmation with character count."""
    mock_exec, mock_process = patch_subprocess

    result = await set_clipboard("hello world")

    assert result == "Clipboard set (11 characters)"
    mock_exec.assert_awaited_once_with(
        "wl-copy",
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.DEVNULL,
        stderr=asyncio.subprocess.DEVNULL,
    )
    mock_process.stdin.write.assert_called_once_with(b"hello world")
    mock_process.stdin.close.assert_called_once()
    mock_process.wait.assert_awaited_once()


async def test_set_clipboard_empty_text(patch_subprocess):
    """set_clipboard() handles empty text."""
    _, mock_process = patch_subprocess

    result = await set_clipboard("")

    assert result == "Clipboard set (0 characters)"
    mock_process.stdin.write.assert_called_once_with(b"")
    mock_process.stdin.close.assert_called_once()


async def test_set_clipboard_unicode(patch_subprocess):
    """set_clipboard() encodes Unicode characters as UTF-8."""
    _, mock_process = patch_subprocess

    result = await set_clipboard("café ☕")

    # 6 Unicode chars → len() returns 6
    assert result == "Clipboard set (6 characters)"
    mock_process.stdin.write.assert_called_once_with("café ☕".encode())


async def test_set_clipboard_timeout(patch_subprocess):
    """set_clipboard() raises TimeoutError when the parent process hangs."""
    _, mock_process = patch_subprocess
    mock_process.wait = AsyncMock(side_effect=asyncio.TimeoutError)

    with pytest.raises(TimeoutError):
        await set_clipboard("some text")
