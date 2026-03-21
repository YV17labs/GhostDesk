# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.clipboard.set_ — write text to clipboard."""

import asyncio
from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.clipboard.set_ import set_clipboard


@pytest.fixture
def mock_process():
    """Create a mock asyncio.subprocess.Process."""
    proc = AsyncMock()
    proc.returncode = 0
    proc.communicate = AsyncMock(return_value=(b"", b""))
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
        "xclip", "-selection", "clipboard", "-i",
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    mock_process.communicate.assert_awaited_once_with(b"hello world")


async def test_set_clipboard_empty_text(patch_subprocess):
    """set_clipboard() handles empty text."""
    _, mock_process = patch_subprocess

    result = await set_clipboard("")

    assert result == "Clipboard set (0 characters)"
    mock_process.communicate.assert_awaited_once_with(b"")


async def test_set_clipboard_xclip_failure(patch_subprocess):
    """set_clipboard() raises RuntimeError when xclip fails."""
    _, mock_process = patch_subprocess
    mock_process.returncode = 1
    mock_process.communicate.return_value = (b"", b"xclip error message")

    with pytest.raises(RuntimeError, match="xclip failed: xclip error message"):
        await set_clipboard("some text")


async def test_set_clipboard_xclip_failure_empty_stderr(patch_subprocess):
    """set_clipboard() raises RuntimeError with empty stderr on failure."""
    _, mock_process = patch_subprocess
    mock_process.returncode = 1
    mock_process.communicate.return_value = (b"", b"")

    with pytest.raises(RuntimeError, match="xclip failed:"):
        await set_clipboard("text")
