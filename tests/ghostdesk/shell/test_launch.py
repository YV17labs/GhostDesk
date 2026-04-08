# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.shell.launch — GUI launcher with process tracking."""

import asyncio
from unittest.mock import AsyncMock, mock_open, patch

import pytest

from ghostdesk.shell.launch import launch

MODULE = "ghostdesk.shell.launch"


@pytest.fixture
def mock_process():
    """Create a mock asyncio.subprocess.Process."""
    proc = AsyncMock()
    proc.pid = 99999
    return proc


@pytest.fixture
def patch_subprocess(mock_process, tmp_path):
    """Patch asyncio.create_subprocess_exec and LOG_DIR."""
    with (
        patch("asyncio.create_subprocess_exec", new_callable=AsyncMock) as mock_exec,
        patch(f"{MODULE}.LOG_DIR", tmp_path),
    ):
        mock_exec.return_value = mock_process
        yield mock_exec, mock_process, tmp_path


async def test_launch_success(patch_subprocess):
    """launch() starts the process and returns pid + log path."""
    mock_exec, mock_proc, tmp_path = patch_subprocess

    result = await launch("firefox https://example.com")

    assert result["pid"] == 99999
    assert result["action"] == "Launched: firefox https://example.com"
    assert "proc-99999.log" in result["log_file"]
    mock_exec.assert_awaited_once()


async def test_launch_creates_log_dir(patch_subprocess):
    """launch() creates the log directory if it doesn't exist."""
    _, _, tmp_path = patch_subprocess
    # tmp_path already exists, but verify the file was created
    result = await launch("firefox")
    log_path = tmp_path / "proc-99999.log"
    assert log_path.exists()


async def test_launch_invalid_syntax():
    """launch() returns error for invalid shell syntax."""
    result = await launch("echo 'unterminated")
    assert "error" in result
    assert "Invalid command syntax" in result["error"]


async def test_launch_empty_command():
    """launch() returns error for empty command string."""
    result = await launch("")
    assert result == {"error": "No command provided"}


async def test_launch_whitespace_only():
    """launch() returns error for whitespace-only command."""
    result = await launch("   ")
    assert result == {"error": "No command provided"}


async def test_launch_file_not_found(patch_subprocess):
    """launch() returns error when the executable is not found."""
    mock_exec, _, tmp_path = patch_subprocess
    mock_exec.side_effect = FileNotFoundError()

    result = await launch("nonexistent_app")

    assert result == {"error": "Command not found: nonexistent_app"}
    # Pending log file should be cleaned up.
    assert not (tmp_path / "proc-pending.log").exists()


async def test_launch_quoted_arguments(patch_subprocess):
    """launch() correctly splits quoted arguments."""
    mock_exec, _, _ = patch_subprocess

    result = await launch('gedit "/tmp/my file.txt"')

    assert result["pid"] == 99999
    assert result["action"] == 'Launched: gedit "/tmp/my file.txt"'
    # Verify the command was split correctly.
    call_args = mock_exec.call_args[0]
    assert call_args == ("gedit", "/tmp/my file.txt")
