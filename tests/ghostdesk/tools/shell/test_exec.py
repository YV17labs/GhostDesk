# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.tools.shell.exec_ — shell command execution."""

import asyncio
import signal
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from ghostdesk.tools.shell.exec_ import shell_exec


@pytest.fixture
def mock_process():
    """Create a mock asyncio.subprocess.Process."""
    proc = AsyncMock()
    proc.pid = 12345
    proc.returncode = 0
    proc.communicate = AsyncMock(return_value=(b"hello world\n", b""))
    proc.stdout = None
    proc.stderr = None
    return proc


@pytest.fixture
def patch_subprocess(mock_process):
    """Patch asyncio.create_subprocess_exec to return mock_process."""
    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock) as mock_exec:
        mock_exec.return_value = mock_process
        yield mock_exec, mock_process


async def test_shell_exec_success(patch_subprocess):
    """shell_exec() returns stdout, stderr, and returncode on success."""
    mock_exec, mock_process = patch_subprocess

    result = await shell_exec("echo hello")

    assert result["stdout"] == "hello world\n"
    assert result["stderr"] == ""
    assert result["returncode"] == 0
    mock_exec.assert_awaited_once_with(
        "bash", "-c", "echo hello",
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        process_group=0,
    )


async def test_shell_exec_command_failure(patch_subprocess):
    """shell_exec() returns non-zero returncode on command failure."""
    _, mock_process = patch_subprocess
    mock_process.returncode = 127
    mock_process.communicate.return_value = (b"", b"command not found\n")

    result = await shell_exec("nonexistent_command")

    assert result["returncode"] == 127
    assert result["stderr"] == "command not found\n"
    assert result["stdout"] == ""


async def test_shell_exec_returncode_none(patch_subprocess):
    """shell_exec() returns 0 when returncode is None."""
    _, mock_process = patch_subprocess
    mock_process.returncode = None

    result = await shell_exec("some_cmd")

    assert result["returncode"] == 0


async def test_shell_exec_timeout_with_kill(patch_subprocess):
    """shell_exec() kills the process group on timeout and drains output."""
    _, mock_process = patch_subprocess

    # First communicate() raises TimeoutError (via wait_for),
    # second communicate() succeeds (after kill).
    call_count = 0
    original_communicate = AsyncMock(return_value=(b"partial", b"err data"))

    async def fake_wait_for(coro, *, timeout):
        nonlocal call_count
        call_count += 1
        if call_count == 1:
            # Cancel the coroutine to avoid "was never awaited" warnings
            coro.close()
            raise asyncio.TimeoutError()
        return await coro

    with patch("asyncio.wait_for", side_effect=fake_wait_for):
        with patch("os.killpg") as mock_killpg:
            mock_process.communicate = original_communicate

            result = await shell_exec("slow_cmd", timeout_seconds=5.0)

            mock_killpg.assert_called_once_with(12345, signal.SIGKILL)
            assert result["returncode"] == -1
            assert "timed out after 5.0s" in result["stderr"]
            assert result["stdout"] == "partial"


async def test_shell_exec_timeout_process_already_gone(patch_subprocess):
    """shell_exec() handles ProcessLookupError when process already exited."""
    _, mock_process = patch_subprocess

    call_count = 0

    async def fake_wait_for(coro, *, timeout):
        nonlocal call_count
        call_count += 1
        if call_count == 1:
            coro.close()
            raise asyncio.TimeoutError()
        return await coro

    mock_process.communicate = AsyncMock(return_value=(b"out", b"err"))

    with patch("asyncio.wait_for", side_effect=fake_wait_for):
        with patch("os.killpg", side_effect=ProcessLookupError):
            result = await shell_exec("gone_cmd", timeout_seconds=2.0)

            assert result["returncode"] == -1
            assert "timed out after 2.0s" in result["stderr"]


async def test_shell_exec_timeout_drain_also_times_out(patch_subprocess):
    """shell_exec() handles drain timeout after kill by closing streams."""
    _, mock_process = patch_subprocess

    mock_stdout = MagicMock()
    mock_stderr = MagicMock()
    mock_process.stdout = mock_stdout
    mock_process.stderr = mock_stderr

    async def fake_wait_for(coro, *, timeout):
        coro.close()
        raise asyncio.TimeoutError()

    mock_process.communicate = AsyncMock(return_value=(b"", b""))

    with patch("asyncio.wait_for", side_effect=fake_wait_for):
        with patch("os.killpg"):
            result = await shell_exec("stuck_cmd", timeout_seconds=1.0)

            mock_stdout.close.assert_called_once()
            mock_stderr.close.assert_called_once()
            assert result["stdout"] == ""
            assert result["stderr"].startswith("Command timed out after 1.0s")
            assert result["returncode"] == -1


async def test_shell_exec_timeout_drain_none_streams(patch_subprocess):
    """shell_exec() handles None streams gracefully during drain timeout."""
    _, mock_process = patch_subprocess
    mock_process.stdout = None
    mock_process.stderr = None

    async def fake_wait_for(coro, *, timeout):
        coro.close()
        raise asyncio.TimeoutError()

    mock_process.communicate = AsyncMock(return_value=(b"", b""))

    with patch("asyncio.wait_for", side_effect=fake_wait_for):
        with patch("os.killpg"):
            result = await shell_exec("stuck_cmd", timeout_seconds=1.0)

            assert result["returncode"] == -1
