# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.utils.cmd — async subprocess runner."""

from unittest.mock import AsyncMock, MagicMock, patch

import asyncio

import pytest

from ghostdesk.utils.cmd import run


@pytest.fixture
def mock_process():
    """Create a mock asyncio.subprocess.Process."""
    proc = AsyncMock()
    proc.returncode = 0
    proc.communicate = AsyncMock(return_value=(b"output text\n", b""))
    proc.kill = MagicMock()
    return proc


@pytest.fixture
def patch_subprocess(mock_process):
    """Patch asyncio.create_subprocess_exec to return mock_process."""
    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock) as mock_exec:
        mock_exec.return_value = mock_process
        yield mock_exec, mock_process


async def test_run_success(patch_subprocess):
    """run() returns stripped stdout on success."""
    mock_exec, mock_process = patch_subprocess
    result = await run(["echo", "hello"])
    assert result == "output text"
    mock_exec.assert_awaited_once_with(
        "echo", "hello",
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )


async def test_run_strips_whitespace(patch_subprocess):
    """run() strips leading/trailing whitespace from stdout."""
    _, mock_process = patch_subprocess
    mock_process.communicate.return_value = (b"  spaced  \n", b"")
    result = await run(["cmd"])
    assert result == "spaced"


async def test_run_nonzero_exit_raises_runtime_error(patch_subprocess):
    """run() raises RuntimeError when process exits with non-zero code."""
    _, mock_process = patch_subprocess
    mock_process.returncode = 1
    mock_process.communicate.return_value = (b"", b"something went wrong\n")

    with pytest.raises(RuntimeError, match="something went wrong"):
        await run(["bad_cmd"])


async def test_run_nonzero_exit_fallback_message(patch_subprocess):
    """run() RuntimeError includes command info when stderr is empty."""
    _, mock_process = patch_subprocess
    mock_process.returncode = 42
    mock_process.communicate.return_value = (b"", b"")

    with pytest.raises(RuntimeError, match="Command failed with code 42"):
        await run(["failing"])


async def test_run_timeout_raises_timeout_error(patch_subprocess):
    """run() raises TimeoutError and kills the process on timeout."""
    _, mock_process = patch_subprocess

    # Make communicate raise asyncio.TimeoutError the first time,
    # then succeed on the second call (after proc.kill()).
    mock_process.communicate = AsyncMock(
        side_effect=[asyncio.TimeoutError(), (b"", b"")]
    )

    with pytest.raises(TimeoutError, match="timed out after 5.0s"):
        await run(["slow_cmd"], timeout=5.0)

    mock_process.kill.assert_called_once()


async def test_run_passes_custom_timeout(patch_subprocess):
    """run() forwards the timeout value to asyncio.wait_for."""
    mock_exec, mock_process = patch_subprocess

    with patch("asyncio.wait_for", new_callable=AsyncMock) as mock_wait_for:
        mock_wait_for.return_value = (b"ok\n", b"")
        mock_process.returncode = 0

        result = await run(["cmd"], timeout=30.0)

        mock_wait_for.assert_awaited_once()
        _, kwargs = mock_wait_for.call_args
        assert kwargs.get("timeout") == 30.0 or mock_wait_for.call_args[0][1] == 30.0
