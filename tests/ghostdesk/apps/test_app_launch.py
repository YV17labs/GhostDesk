# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.apps.app_launch — GUI launcher with process tracking."""

import asyncio
from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.apps.app_launch import _launched_pids, app_launch

MODULE = "ghostdesk.apps.app_launch"


@pytest.fixture(autouse=True)
def _clear_pid_registry():
    """Ensure the PID registry is clean before and after every test."""
    _launched_pids.clear()
    yield
    _launched_pids.clear()


@pytest.fixture
def mock_process():
    """Create a mock asyncio.subprocess.Process."""
    proc = AsyncMock()
    proc.pid = 99999
    return proc


@pytest.fixture
def patch_subprocess(mock_process, tmp_path):
    """Patch subprocess, LOG_DIR, and whitelist to allow 'firefox'."""
    with (
        patch("asyncio.create_subprocess_exec", new_callable=AsyncMock) as mock_exec,
        patch(f"{MODULE}.LOG_DIR", tmp_path),
        patch(f"{MODULE}.known_executables", return_value=frozenset({"firefox", "gedit"})),
    ):
        mock_exec.return_value = mock_process
        yield mock_exec, mock_process, tmp_path


# --- whitelist ---

async def test_app_launch_rejects_unknown_executable():
    """app_launch() rejects commands not in the GUI app whitelist."""
    with patch(f"{MODULE}.known_executables", return_value=frozenset({"firefox"})):
        result = await app_launch("rm -rf /")
    assert "error" in result
    assert "not a known gui app" in result["error"].lower()


async def test_app_launch_rejects_full_path_to_unknown():
    """app_launch() rejects full-path commands not in the whitelist."""
    with patch(f"{MODULE}.known_executables", return_value=frozenset({"firefox"})):
        result = await app_launch("/usr/bin/bash")
    assert "error" in result


async def test_app_launch_accepts_full_path_to_known(patch_subprocess):
    """app_launch() accepts /usr/bin/firefox when 'firefox' is whitelisted."""
    result = await app_launch("/usr/bin/firefox")
    assert "pid" in result


# --- PID registry ---

async def test_app_launch_registers_pid(patch_subprocess):
    """app_launch() adds the new PID to _launched_pids."""
    await app_launch("firefox")
    assert 99999 in _launched_pids


async def test_app_launch_does_not_register_on_failure():
    """app_launch() does not register a PID when the command is rejected."""
    with patch(f"{MODULE}.known_executables", return_value=frozenset()):
        await app_launch("firefox")
    assert not _launched_pids


# --- standard launch behaviour ---

async def test_app_launch_success(patch_subprocess):
    """app_launch() starts the process and returns pid + log path."""
    mock_exec, _, _ = patch_subprocess
    result = await app_launch("firefox https://example.com")
    assert result["pid"] == 99999
    assert result["action"] == "Launched: firefox https://example.com"
    assert "proc-99999.log" in result["log_file"]
    mock_exec.assert_awaited_once()


async def test_app_launch_creates_log_file(patch_subprocess):
    """app_launch() creates the log file under LOG_DIR."""
    _, _, tmp_path = patch_subprocess
    await app_launch("firefox")
    assert (tmp_path / "proc-99999.log").exists()


async def test_app_launch_invalid_syntax():
    """app_launch() returns error for invalid shell syntax."""
    result = await app_launch("echo 'unterminated")
    assert "error" in result
    assert "Invalid command syntax" in result["error"]


async def test_app_launch_empty_command():
    """app_launch() returns error for empty command string."""
    result = await app_launch("")
    assert result == {"error": "No command provided"}


async def test_app_launch_whitespace_only():
    """app_launch() returns error for whitespace-only command."""
    result = await app_launch("   ")
    assert result == {"error": "No command provided"}


async def test_app_launch_file_not_found(patch_subprocess):
    """app_launch() returns error when the executable is not found on disk."""
    mock_exec, _, tmp_path = patch_subprocess
    mock_exec.side_effect = FileNotFoundError()
    result = await app_launch("firefox")
    assert "error" in result
    assert "Command not found" in result["error"]
    assert 99999 not in _launched_pids


async def test_app_launch_quoted_arguments(patch_subprocess):
    """app_launch() correctly splits quoted arguments."""
    mock_exec, _, _ = patch_subprocess
    result = await app_launch('gedit "/tmp/my file.txt"')
    assert result["pid"] == 99999
    assert mock_exec.call_args[0] == ("gedit", "/tmp/my file.txt")
