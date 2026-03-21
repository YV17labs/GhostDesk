# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.shell.launch — fire-and-forget GUI launcher."""

import asyncio
from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.shell.launch import launch


@pytest.fixture
def mock_process():
    """Create a mock asyncio.subprocess.Process."""
    proc = AsyncMock()
    proc.pid = 99999
    return proc


@pytest.fixture
def patch_subprocess(mock_process):
    """Patch asyncio.create_subprocess_exec to return mock_process."""
    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock) as mock_exec:
        mock_exec.return_value = mock_process
        yield mock_exec, mock_process


async def test_launch_success(patch_subprocess):
    """launch() starts the process and returns a confirmation message."""
    mock_exec, _ = patch_subprocess

    result = await launch("firefox https://example.com")

    assert result == "Launched: firefox https://example.com"
    mock_exec.assert_awaited_once_with(
        "firefox", "https://example.com",
        stdout=asyncio.subprocess.DEVNULL,
        stderr=asyncio.subprocess.DEVNULL,
        process_group=0,
    )


async def test_launch_invalid_syntax():
    """launch() returns error message for invalid shell syntax (bad quotes)."""
    result = await launch("echo 'unterminated")

    assert "Invalid command syntax" in result


async def test_launch_empty_command():
    """launch() returns error for empty command string."""
    result = await launch("")

    assert result == "No command provided"


async def test_launch_whitespace_only():
    """launch() returns error for whitespace-only command."""
    result = await launch("   ")

    assert result == "No command provided"


async def test_launch_file_not_found(patch_subprocess):
    """launch() returns error when the executable is not found."""
    mock_exec, _ = patch_subprocess
    mock_exec.side_effect = FileNotFoundError()

    result = await launch("nonexistent_app")

    assert result == "Command not found: nonexistent_app"


async def test_launch_quoted_arguments(patch_subprocess):
    """launch() correctly splits quoted arguments."""
    mock_exec, _ = patch_subprocess

    result = await launch('gedit "/tmp/my file.txt"')

    assert result == 'Launched: gedit "/tmp/my file.txt"'
    mock_exec.assert_awaited_once_with(
        "gedit", "/tmp/my file.txt",
        stdout=asyncio.subprocess.DEVNULL,
        stderr=asyncio.subprocess.DEVNULL,
        process_group=0,
    )
